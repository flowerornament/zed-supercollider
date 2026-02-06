//! LSP protocol bridge between stdin/stdout and UDP.
//!
//! Handles the bidirectional communication between Zed (via stdio) and
//! sclang's LanguageServer.quark (via UDP).

use anyhow::{Context, Result};
use log::{debug, error, warn};
use lsp_types::{
    CodeActionKind, CodeActionProviderCapability, CodeLensOptions, CompletionOptions,
    CompletionOptionsCompletionItem, DeclarationCapability, ExecuteCommandOptions,
    FoldingRangeProviderCapability, HoverProviderCapability, ImplementationProviderCapability,
    InitializeResult, OneOf, SaveOptions, SelectionRangeProviderCapability, ServerCapabilities,
    ServerInfo, SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::constants::*;
use crate::logging::{debug_file_logs_enabled, log_dir, timestamp};

// ============================================================================
// Request ID Types
// ============================================================================

/// Type-safe request ID representation supporting both number and string IDs.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    /// Extract a RequestId from a JSON value.
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        match value {
            JsonValue::Number(n) => n.as_i64().map(RequestId::Number),
            JsonValue::String(s) => Some(RequestId::String(s.clone())),
            _ => None,
        }
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

// ============================================================================
// LSP Request ID Counter
// ============================================================================

/// Global request ID counter for launcher-originated LSP requests.
static NEXT_LSP_REQUEST_ID: AtomicU64 = AtomicU64::new(INITIAL_LSP_REQUEST_ID);

/// Get the next LSP request ID for launcher-originated requests.
pub fn next_lsp_request_id() -> u64 {
    NEXT_LSP_REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// LSP Message Parsing Helpers
// ============================================================================

/// Extract LSP method and JSON body from a raw LSP message.
/// Returns None if the message cannot be parsed or has no method field.
fn extract_lsp_info(message: &[u8]) -> Option<(JsonValue, String)> {
    let body_str = std::str::from_utf8(message).ok()?;
    let body_start = body_str.find("\r\n\r\n")?;
    let body = &body_str[body_start + 4..];
    let json: JsonValue = serde_json::from_str(body).ok()?;
    let method = json.get("method")?.as_str()?.to_string();
    Some((json, method))
}

/// Ensure JSON-RPC response has the required "jsonrpc": "2.0" field.
/// Returns the patched body if modification was needed, None otherwise.
fn patch_jsonrpc_version(body: &[u8]) -> Option<Vec<u8>> {
    let mut value: JsonValue = serde_json::from_slice(body).ok()?;
    if value.get("jsonrpc").is_some() {
        return None; // Already has jsonrpc field
    }
    let JsonValue::Object(ref mut map) = value else {
        return None;
    };
    map.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    serde_json::to_vec(&value).ok()
}

/// Check if a response should be suppressed (we already responded to this request ID).
fn should_suppress_response(body: &[u8], responded_ids: &Mutex<HashSet<RequestId>>) -> bool {
    let Ok(json) = serde_json::from_slice::<JsonValue>(body) else {
        return false;
    };
    let Some(id) = json.get("id") else {
        return false;
    };
    let Some(request_id) = RequestId::from_json(id) else {
        return false;
    };
    let Ok(set) = responded_ids.lock() else {
        return false;
    };
    if set.contains(&request_id) {
        debug!(
            "SUPPRESSING duplicate response for id={} (already responded from launcher)",
            request_id
        );
        true
    } else {
        false
    }
}

/// Context for handling an initialize request.
struct InitializeContext<'a> {
    json: &'a JsonValue,
    message: &'a [u8],
    responded_ids: &'a Mutex<HashSet<RequestId>>,
    cached_initialize: &'a Mutex<Option<Vec<u8>>>,
    stdin_log: &'a mut Option<std::fs::File>,
}

/// Handle an LSP initialize request by responding immediately.
/// Zed expects a fast response; we can't wait for sclang.
fn handle_initialize_request(ctx: InitializeContext<'_>) {
    let Some(id) = ctx.json.get("id") else {
        return;
    };

    debug!("INTERCEPTING initialize request - responding immediately");

    let response = create_initialize_response(id.clone());
    let response_json =
        serde_json::to_string(&response).expect("initialize response must serialize");
    let response_msg = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_json.len(),
        response_json
    );

    // Write directly to stdout
    let mut stdout = io::stdout();
    if let Err(e) = stdout.write_all(response_msg.as_bytes()) {
        error!("failed to write initialize response: {}", e);
    }
    if let Err(e) = stdout.flush() {
        error!("failed to flush initialize response: {}", e);
    }
    debug!("sent initialize response to Zed");

    // Log to file
    if let Some(ref mut f) = ctx.stdin_log {
        let _ = writeln!(
            f,
            "[{}] >>> RESPONDED TO INITIALIZE: {}",
            timestamp(),
            response_json
        );
    }

    // Cache initialize for re-sending after recompile
    if let Ok(mut slot) = ctx.cached_initialize.lock() {
        *slot = Some(ctx.message.to_vec());
    }

    // Record that we've already responded to this request ID
    // so we can suppress sclang's duplicate response
    let Some(request_id) = RequestId::from_json(id) else {
        return;
    };
    let Ok(mut set) = ctx.responded_ids.lock() else {
        return;
    };
    set.insert(request_id.clone());
    debug!("recorded responded id={} for suppression", request_id);
}

// ============================================================================
// LSP Message Creation
// ============================================================================

/// Create an LSP initialize response with server capabilities.
/// This is sent immediately by the launcher so Zed doesn't timeout waiting for sclang.
pub fn create_initialize_response(id: JsonValue) -> JsonValue {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: None,
                })),
            },
        )),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".into(), "(".into(), "~".into()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: Some(CompletionOptionsCompletionItem {
                label_details_support: Some(true),
            }),
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".into()]),
            retrigger_characters: Some(vec![",".into()]),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: None,
        }),
        code_action_provider: Some(CodeActionProviderCapability::Options(
            lsp_types::CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::SOURCE]),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
                resolve_provider: None,
            },
        )),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![
                "supercollider.eval".into(),
                "supercollider.evaluateSelection".into(),
                "supercollider.internal.bootServer".into(),
                "supercollider.internal.rebootServer".into(),
                "supercollider.internal.quitServer".into(),
                "supercollider.internal.recompile".into(),
                "supercollider.internal.cmdPeriod".into(),
                "supercollider.internal.openPostLog".into(),
            ],
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        }),
        ..Default::default()
    };

    let result = InitializeResult {
        capabilities,
        server_info: Some(ServerInfo {
            name: "sclang:LSPConnection".into(),
            version: Some("0.1".into()),
        }),
    };

    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result
    })
}

/// Create a typed LSP request with automatic JSON-RPC envelope.
pub fn create_lsp_request<P: serde::Serialize>(id: u64, method: &str, params: P) -> JsonValue {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "method": method,
        "params": params
    })
}

/// Create a typed LSP notification (no id field).
pub fn create_lsp_notification<P: serde::Serialize>(method: &str, params: P) -> JsonValue {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params
    })
}

/// Create a workspace/executeCommand request with type-safe arguments.
pub fn create_execute_command_request(
    id: u64,
    command: &str,
    arguments: Vec<JsonValue>,
) -> JsonValue {
    let params = lsp_types::ExecuteCommandParams {
        command: command.to_string(),
        arguments,
        work_done_progress_params: Default::default(),
    };
    create_lsp_request(id, "workspace/executeCommand", params)
}

// ============================================================================
// UDP Send/Receive
// ============================================================================

/// Send a message to sclang via UDP with retry on connection refused.
pub fn send_with_retry(socket: &UdpSocket, message: &[u8]) -> io::Result<()> {
    use std::io::ErrorKind;

    let mut attempts = 0usize;
    let max_attempts = max_retry_attempts();

    // Log what we're sending (extract method if possible)
    if let Some((json, method)) = extract_lsp_info(message) {
        debug!(
            ">>> SENDING to sclang: method={} id={:?} size={}",
            method,
            json.get("id"),
            message.len()
        );
    }

    // If message fits in one packet, send directly
    if message.len() <= MAX_UDP_CHUNK_SIZE {
        loop {
            match socket.send(message) {
                Ok(bytes) if bytes == message.len() => return Ok(()),
                Ok(_) => {
                    return Err(io::Error::other(
                        "partial UDP send (wrote fewer bytes than expected)",
                    ))
                }
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    if attempts == 0 || attempts.is_multiple_of(40) {
                        debug!(
                            "Connection refused sending to sclang (attempt {}): {err}",
                            attempts + 1
                        );
                    }
                    if attempts >= max_attempts {
                        return Err(io::Error::new(
                            ErrorKind::ConnectionRefused,
                            format!(
                                "connection refused after {} retries (~{}s): {err}",
                                attempts + 1,
                                MAX_RETRY_MS / 1000
                            ),
                        ));
                    }
                    std::thread::sleep(millis_to_duration(RETRY_SLEEP_MS));
                    attempts += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    // Large message: chunk it like LanguageServer.quark does
    debug!(
        "chunking large message ({} bytes) into {} chunks",
        message.len(),
        message.len().div_ceil(MAX_UDP_CHUNK_SIZE)
    );

    let mut offset = 0;
    while offset < message.len() {
        let end = (offset + MAX_UDP_CHUNK_SIZE).min(message.len());
        let chunk = &message[offset..end];

        loop {
            match socket.send(chunk) {
                Ok(bytes) if bytes == chunk.len() => break,
                Ok(_) => return Err(io::Error::other("partial UDP send on chunk")),
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    if attempts == 0 || attempts.is_multiple_of(40) {
                        debug!(
                            "Connection refused sending chunk (attempt {}): {err}",
                            attempts + 1
                        );
                    }
                    if attempts >= max_attempts {
                        return Err(io::Error::new(
                            ErrorKind::ConnectionRefused,
                            format!("connection refused after {} retries: {err}", attempts + 1),
                        ));
                    }
                    std::thread::sleep(millis_to_duration(RETRY_SLEEP_MS));
                    attempts += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
        offset = end;
        // Small delay between chunks to avoid overwhelming the receiver
        std::thread::sleep(Duration::from_micros(UDP_CHUNK_DELAY_US));
    }

    Ok(())
}

/// Read an LSP message from a buffered reader.
/// Returns None on EOF, Some(message) on success, Err on parse error.
pub fn read_lsp_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut raw_lines: Vec<Vec<u8>> = Vec::new();
    let mut header_buffer = String::new();

    loop {
        header_buffer.clear();
        let bytes = reader.read_line(&mut header_buffer)?;
        if bytes == 0 {
            if raw_lines.is_empty() {
                return Ok(None);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unexpected EOF while reading LSP headers",
                ));
            }
        }

        let line_bytes = header_buffer.as_bytes().to_vec();
        let trimmed = header_buffer.trim_end_matches(&['\r', '\n'][..]);

        if trimmed.is_empty() {
            raw_lines.push(line_bytes);
            break;
        }

        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let len = rest.trim().parse::<usize>().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length header: {err}"),
                )
            })?;
            content_length = Some(len);
        }

        raw_lines.push(line_bytes);
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header in LSP message",
        )
    })?;

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    let mut message = Vec::new();
    for line in raw_lines {
        message.extend_from_slice(&line);
    }
    message.extend_from_slice(&body);

    Ok(Some(message))
}

// ============================================================================
// Message Cache and Flush Helpers
// ============================================================================

/// Try to get a cached message and send it via UDP.
/// Logs errors but doesn't propagate them.
fn try_send_cached(cache: &Mutex<Option<Vec<u8>>>, socket: &UdpSocket, msg_name: &str) {
    let Some(msg) = cache.lock().ok().and_then(|m| m.clone()) else {
        return;
    };
    if let Err(err) = send_with_retry(socket, &msg) {
        error!("failed to re-send {}: {err}", msg_name);
    }
}

// ============================================================================
// Sender Thread Context and Helpers
// ============================================================================

/// Shared state for the sender thread.
struct SenderContext {
    socket: UdpSocket,
    ready: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    ready_count: Arc<AtomicU64>,
    cached_initialize: Arc<Mutex<Option<Vec<u8>>>>,
    cached_did_open: Arc<Mutex<Option<Vec<u8>>>>,
    cached_did_change: Arc<Mutex<Option<Vec<u8>>>>,
}

/// Handle the transition from not-ready to ready state.
/// Returns true if the transition occurred this call.
fn handle_ready_transition(
    ctx: &SenderContext,
    pending: &mut Vec<Vec<u8>>,
    start_time: std::time::Instant,
) -> bool {
    if !ctx.ready.load(Ordering::SeqCst) {
        return false;
    }

    debug!(
        "sender thread: sclang ready at t={}ms, {} messages buffered",
        start_time.elapsed().as_millis(),
        pending.len()
    );

    // Re-send cached didOpen/didChange after providers are likely registered.
    if let Some(open_msg) = ctx.cached_did_open.lock().ok().and_then(|m| m.clone()) {
        debug!("re-sending cached textDocument/didOpen after sclang ready");
        pending.push(open_msg);
    }
    if let Some(change_msg) = ctx.cached_did_change.lock().ok().and_then(|m| m.clone()) {
        debug!("re-sending cached textDocument/didChange after sclang ready");
        pending.push(change_msg);
    }

    flush_pending_messages(pending, &ctx.socket, start_time);
    true
}

/// Flush all pending messages to the socket.
fn flush_pending_messages(
    pending: &mut Vec<Vec<u8>>,
    socket: &UdpSocket,
    start_time: std::time::Instant,
) {
    if pending.is_empty() {
        return;
    }

    debug!(
        "flushing {} buffered messages at t={}ms",
        pending.len(),
        start_time.elapsed().as_millis()
    );

    for msg in pending.drain(..) {
        if let Err(err) = send_with_retry(socket, &msg) {
            error!("failed to send buffered UDP message: {err}");
        }
    }
}

/// Handle recompile detection by checking if ready count increased.
/// Returns the new last_ready_count value.
fn handle_recompile_check(ctx: &SenderContext, last_ready_count: u64) -> u64 {
    let current = ctx.ready_count.load(Ordering::SeqCst);
    if current > last_ready_count {
        if last_ready_count > 0 {
            // This is a recompile (not the initial ready)
            debug!(
                "RECOMPILE DETECTED (ready count {} -> {}), re-sending state",
                last_ready_count, current
            );
            try_send_cached(&ctx.cached_initialize, &ctx.socket, "initialize");
            try_send_cached(&ctx.cached_did_open, &ctx.socket, "didOpen");
            try_send_cached(&ctx.cached_did_change, &ctx.socket, "didChange");
        }
        current
    } else {
        last_ready_count
    }
}

/// Handle shutdown flush with deadline waiting for ready signal.
fn handle_shutdown_flush(ctx: &SenderContext, pending: &mut Vec<Vec<u8>>, ready_signaled: bool) {
    if pending.is_empty() {
        return;
    }

    if ready_signaled {
        // sclang is ready, flush all pending messages
        for msg in pending.drain(..) {
            if let Err(e) = send_with_retry(&ctx.socket, &msg) {
                debug!("send failed during shutdown flush: {}", e);
            }
        }
        return;
    }

    // sclang not ready - wait briefly for ready signal
    let deadline = std::time::Instant::now() + millis_to_duration(SHUTDOWN_FLUSH_WAIT_MS);
    while std::time::Instant::now() < deadline {
        if ctx.ready.load(Ordering::SeqCst) {
            for msg in pending.drain(..) {
                if let Err(e) = send_with_retry(&ctx.socket, &msg) {
                    debug!("send failed during deadline flush: {}", e);
                }
            }
            return;
        }
        std::thread::sleep(millis_to_duration(STARTUP_POLL_MS));
    }

    if !pending.is_empty() {
        warn!(
            "dropping {} messages - sclang never became ready",
            pending.len()
        );
    }
}

/// Main sender thread loop - buffers messages until sclang is ready, then sends.
fn run_sender_thread(ctx: SenderContext, msg_rx: mpsc::Receiver<Vec<u8>>) {
    let start_time = std::time::Instant::now();
    let mut pending: Vec<Vec<u8>> = Vec::new();
    let mut ready_signaled = false;
    let mut last_ready_count: u64 = 0;

    loop {
        // Check for recompile
        last_ready_count = handle_recompile_check(&ctx, last_ready_count);

        // Check for ready signal transition
        if !ready_signaled && handle_ready_transition(&ctx, &mut pending, start_time) {
            ready_signaled = true;
        }

        // Try to receive a message (with timeout to allow checking ready flag)
        match msg_rx.recv_timeout(millis_to_duration(STARTUP_POLL_MS)) {
            Ok(message) => {
                if ready_signaled {
                    if let Err(err) = send_with_retry(&ctx.socket, &message) {
                        error!("failed to send UDP message: {err}");
                    }
                } else {
                    pending.push(message);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Continue checking ready flag
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Reader thread closed
                handle_shutdown_flush(&ctx, &mut pending, ready_signaled);
                break;
            }
        }

        if ctx.shutdown.load(Ordering::SeqCst) {
            // Drain remaining messages from channel
            while let Ok(message) = msg_rx.try_recv() {
                if ready_signaled {
                    if let Err(e) = send_with_retry(&ctx.socket, &message) {
                        debug!("send failed during shutdown drain: {}", e);
                    }
                } else {
                    pending.push(message);
                }
            }
            // Final flush
            if ready_signaled {
                for msg in pending.drain(..) {
                    if let Err(e) = send_with_retry(&ctx.socket, &msg) {
                        debug!("send failed during final flush: {}", e);
                    }
                }
            } else if !pending.is_empty() {
                warn!(
                    "dropping {} messages on shutdown (sclang not ready)",
                    pending.len()
                );
            }
            break;
        }
    }
}

// ============================================================================
// Stdin → UDP Bridge
// ============================================================================

/// Bridge stdin to UDP, forwarding LSP messages from Zed to sclang.
/// Handles initialize request interception, message buffering, and recompile detection.
#[allow(clippy::too_many_arguments)]
pub fn pump_stdin_to_udp(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
    done_tx: mpsc::Sender<()>,
    sclang_ready: Arc<AtomicBool>,
    responded_ids: Arc<Mutex<HashSet<RequestId>>>,
    ready_count: Arc<AtomicU64>,
) -> Result<()> {
    // Cache the most recent didOpen/didChange to resend after providers register.
    let cached_did_open: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let cached_did_change: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    // Cache initialize request to resend after recompile
    let cached_initialize: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    let mut stdin_log = if debug_file_logs_enabled() {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir().join("sc_launcher_stdin.log"))
            .ok()
    } else {
        None
    };
    if let Some(ref mut f) = stdin_log {
        let _ = writeln!(f, "\n[{}] === pump_stdin_to_udp ENTERED ===", timestamp());
    }

    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());

    // Use a channel to queue messages for sending (allows separate flush thread)
    let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>();

    // Build sender context with all shared state
    let ctx = SenderContext {
        socket: socket
            .try_clone()
            .context("failed to clone socket for sender")?,
        ready: sclang_ready.clone(),
        shutdown: shutdown.clone(),
        ready_count: ready_count.clone(),
        cached_initialize: cached_initialize.clone(),
        cached_did_open: cached_did_open.clone(),
        cached_did_change: cached_did_change.clone(),
    };

    // Spawn sender thread
    let sender_thread = thread::Builder::new()
        .name("stdin-sender".into())
        .spawn(move || run_sender_thread(ctx, msg_rx))?;

    if let Some(ref mut f) = stdin_log {
        let _ = writeln!(f, "[{}] stdin reader: starting main loop", timestamp());
    }
    let mut msg_count = 0u64;

    // Main loop: read from stdin and queue messages to sender thread
    while !shutdown.load(Ordering::SeqCst) {
        match read_lsp_message(&mut reader) {
            Ok(Some(message)) => {
                msg_count += 1;
                // Log to file
                if let Some(ref mut f) = stdin_log {
                    let preview = String::from_utf8_lossy(&message[..message.len().min(500)]);
                    let _ = writeln!(
                        f,
                        "[{}] MSG#{} ({} bytes): {}",
                        timestamp(),
                        msg_count,
                        message.len(),
                        preview
                    );
                }
                // Log incoming LSP method for debugging and handle initialize specially
                let is_buffered = !sclang_ready.load(Ordering::SeqCst);

                if let Some((json, method)) = extract_lsp_info(&message) {
                    debug!(
                        "<< LSP request: {} (id={:?}) size={} {}",
                        method,
                        json.get("id"),
                        message.len(),
                        if is_buffered { "[BUFFERED]" } else { "" }
                    );

                    // Cache last didOpen/didChange so we can replay after sclang is ready
                    match method.as_str() {
                        "textDocument/didOpen" => {
                            if let Ok(mut slot) = cached_did_open.lock() {
                                *slot = Some(message.clone());
                            }
                        }
                        "textDocument/didChange" => {
                            if let Ok(mut slot) = cached_did_change.lock() {
                                *slot = Some(message.clone());
                            }
                        }
                        "initialize" => {
                            // Handle initialize request IMMEDIATELY from the launcher
                            // We can't wait for sclang because Zed expects a fast response
                            handle_initialize_request(InitializeContext {
                                json: &json,
                                message: &message,
                                responded_ids: &responded_ids,
                                cached_initialize: &cached_initialize,
                                stdin_log: &mut stdin_log,
                            });
                        }
                        _ => {}
                    }
                }

                // Queue message for sender thread (forward to sclang)
                if msg_tx.send(message).is_err() {
                    error!("sender thread closed unexpectedly");
                    break;
                }
            }
            Ok(None) => {
                // graceful shutdown (stdin closed)
                let _ = done_tx.send(());
                break;
            }
            Err(err) => {
                error!("error reading from stdin: {err}");
                let _ = done_tx.send(());
                break;
            }
        }
    }

    // Clean up sender thread
    drop(msg_tx);
    let _ = sender_thread.join();

    shutdown.store(true, Ordering::SeqCst);
    Ok(())
}

// ============================================================================
// UDP → Stdout Bridge
// ============================================================================

/// Bridge UDP to stdout, forwarding LSP messages from sclang to Zed.
/// Handles message reassembly, JSON-RPC patching, and duplicate response suppression.
pub fn pump_udp_to_stdout(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
    responded_ids: Arc<Mutex<HashSet<RequestId>>>,
) -> Result<()> {
    debug!("UDP->stdout bridge listening on {:?}", socket.local_addr());
    let mut dgram_buf = vec![0u8; UDP_BUFFER_SIZE];
    let mut stdout = io::stdout();

    // Accumulator for potentially fragmented UDP messages coming from sclang.
    let mut acc: Vec<u8> = Vec::new();
    let mut expected_len: Option<usize> = None;

    // Helper to try parsing a Content-Length header from the accumulator.
    #[inline]
    fn try_parse_header(buf: &[u8]) -> Option<(usize /* body_start */, usize /* len */)> {
        let hay = buf;
        let cl = b"Content-Length:";
        let hdr_start = hay.windows(cl.len()).position(|w| w == cl)?;
        let after = &hay[hdr_start + cl.len()..];
        // Skip optional spaces
        let mut i = 0usize;
        while i < after.len() && (after[i] == b' ' || after[i] == b'\t') {
            i += 1;
        }
        // Parse digits
        let mut len: usize = 0;
        let mut saw_digit = false;
        while i < after.len() {
            let b = after[i];
            if (b as char).is_ascii_digit() {
                saw_digit = true;
                len = len.saturating_mul(10).saturating_add((b - b'0') as usize);
                i += 1;
            } else {
                break;
            }
        }
        if !saw_digit {
            return None;
        }
        // Find end of header sequence \r\n\r\n
        if let Some(hdr_end_rel) = after[i..].windows(4).position(|w| w == b"\r\n\r\n") {
            let body_start = hdr_start + cl.len() + i + hdr_end_rel + 4;
            Some((body_start, len))
        } else {
            None
        }
    }

    while !shutdown.load(Ordering::SeqCst) {
        match socket.recv(&mut dgram_buf) {
            Ok(size) => {
                if size == 0 {
                    continue;
                }
                acc.extend_from_slice(&dgram_buf[..size]);

                // Process as many complete messages as are buffered.
                'outer: loop {
                    if expected_len.is_none() {
                        if let Some((body_start, len)) = try_parse_header(&acc) {
                            // Drop header, keep only body and any following bytes.
                            acc.drain(0..body_start);
                            expected_len = Some(len);
                        } else {
                            // Need more header bytes.
                            break 'outer;
                        }
                    }

                    if let Some(len) = expected_len {
                        if acc.len() < len {
                            // Need more body bytes.
                            break 'outer;
                        }

                        // Split out one complete body.
                        let mut body: Vec<u8> = acc.drain(0..len).collect();
                        expected_len = None;

                        // Ensure JSON-RPC responses include the required jsonrpc version tag.
                        if let Some(patched_body) = patch_jsonrpc_version(&body) {
                            body = patched_body;
                            debug!("patched missing jsonrpc field in server message");
                        }

                        // Check if this is a response to a request we've already handled
                        // (e.g., initialize response from sclang when we already responded)
                        if should_suppress_response(&body, &responded_ids) {
                            continue 'outer;
                        }

                        // Write exactly one LSP message to stdout, potentially patched.
                        let header = format!("Content-Length: {}\r\n\r\n", body.len());
                        if let Err(err) = stdout.write_all(header.as_bytes()) {
                            error!("failed to write header: {err}");
                            break;
                        }
                        if let Err(err) = stdout.write_all(&body) {
                            error!("failed to write LSP body: {err}");
                            break;
                        }
                        if let Err(err) = stdout.flush() {
                            error!("failed to flush stdout: {err}");
                            break;
                        }
                        // If the accumulator still contains more bytes, loop to parse them.
                        continue 'outer;
                    }
                }
            }
            Err(err)
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.kind() == io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(err) => {
                error!("UDP receive error: {err}");
                break;
            }
        }
    }

    shutdown.store(true, Ordering::SeqCst);
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_from_json_number() {
        let json = serde_json::json!(42);
        let id = RequestId::from_json(&json);
        assert!(matches!(id, Some(RequestId::Number(42))));
    }

    #[test]
    fn test_request_id_from_json_string() {
        let json = serde_json::json!("abc-123");
        let id = RequestId::from_json(&json);
        assert!(matches!(id, Some(RequestId::String(ref s)) if s == "abc-123"));
    }

    #[test]
    fn test_request_id_from_json_null() {
        let json = serde_json::json!(null);
        let id = RequestId::from_json(&json);
        assert!(id.is_none());
    }

    #[test]
    fn test_request_id_display() {
        let num = RequestId::Number(42);
        assert_eq!(format!("{}", num), "42");

        let string = RequestId::String("test".into());
        assert_eq!(format!("{}", string), "\"test\"");
    }

    #[test]
    fn test_next_lsp_request_id_increments() {
        let id1 = next_lsp_request_id();
        let id2 = next_lsp_request_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_create_initialize_response_has_capabilities() {
        let response = create_initialize_response(serde_json::json!(1));
        assert!(response.get("result").is_some());
        assert!(response
            .get("result")
            .unwrap()
            .get("capabilities")
            .is_some());
    }

    #[test]
    fn test_create_lsp_request_format() {
        let request = create_lsp_request(1, "test/method", serde_json::json!({"key": "value"}));
        assert_eq!(request.get("jsonrpc").unwrap(), "2.0");
        assert_eq!(request.get("id").unwrap(), 1);
        assert_eq!(request.get("method").unwrap(), "test/method");
    }

    #[test]
    fn test_create_lsp_notification_no_id() {
        let notification = create_lsp_notification("test/notify", serde_json::json!({}));
        assert_eq!(notification.get("jsonrpc").unwrap(), "2.0");
        assert!(notification.get("id").is_none());
        assert_eq!(notification.get("method").unwrap(), "test/notify");
    }

    #[test]
    fn test_read_lsp_message_valid() {
        let message = "Content-Length: 13\r\n\r\n{\"test\":true}";
        let mut reader = io::BufReader::new(message.as_bytes());
        let result = read_lsp_message(&mut reader).unwrap();
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(String::from_utf8_lossy(&msg).contains("test"));
    }

    #[test]
    fn test_read_lsp_message_eof() {
        let message = "";
        let mut reader = io::BufReader::new(message.as_bytes());
        let result = read_lsp_message(&mut reader).unwrap();
        assert!(result.is_none());
    }
}
