use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tiny_http::{Method, Response, Server};

/// SuperCollider Language Server launcher
///
/// Responsibilities:
/// - Detect sclang path.
/// - Warn when LanguageServer.quark is absent.
/// - Launch sclang with LanguageServer enabled and bridge UDP↔stdio.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    /// Probe sclang availability and print JSON
    Probe,
    /// Run the LSP bridge (stdin/stdout ↔ LanguageServer.quark UDP transport)
    Lsp,
}

#[derive(Parser, Debug)]
#[command(name = "sc_launcher", version, about = "Launch sclang LSP for Zed")]
struct Args {
    /// Path to sclang executable (overrides detection)
    #[arg(long)]
    sclang_path: Option<String>,

    /// Optional SuperCollider config YAML path
    #[arg(long)]
    conf_yaml_path: Option<String>,

    /// Launcher mode
    #[arg(long, value_enum, default_value_t = Mode::Probe)]
    mode: Mode,

    /// Optional LSP log level forwarded to LanguageServer.quark (e.g. error, warn, info, debug)
    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// HTTP server port for eval requests (0 = auto-assign, default 57130)
    #[arg(long, default_value_t = 57130)]
    http_port: u16,
}

fn log_dir() -> std::path::PathBuf {
    std::env::var_os("SC_TMP_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
}

fn debug_file_logs_enabled() -> bool {
    std::env::var("SC_LAUNCHER_DEBUG_LOGS").is_ok()
}

fn verbose_logging_enabled() -> bool {
    debug_file_logs_enabled() || std::env::var("SC_LAUNCHER_DEBUG").is_ok()
}

fn main() -> Result<()> {
    // Write startup log to a file since stderr may be buffered/filtered by Zed
    if debug_file_logs_enabled() {
        let log_path = log_dir().join("sc_launcher_startup.log");
        let startup_log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path);
        if let Ok(mut f) = startup_log {
            use std::io::Write;
            let _ = writeln!(
                f,
                "\n======== MAIN STARTED at {:?} ========",
                std::time::SystemTime::now()
            );
            let _ = writeln!(
                f,
                "PID={} args={:?}",
                std::process::id(),
                std::env::args().collect::<Vec<_>>()
            );
            let _ = writeln!(f, "exe={:?}", std::env::current_exe());
            let _ = writeln!(f, "log_dir={:?}", log_dir());
        }
    }

    // Also try stderr
    eprintln!("[sc_launcher] ======== MAIN STARTED ========");
    eprintln!(
        "[sc_launcher] PID={} args={:?}",
        std::process::id(),
        std::env::args().collect::<Vec<_>>()
    );
    let _ = std::io::stderr().flush();

    let args = Args::parse();

    let sclang = match &args.sclang_path {
        Some(p) => p.clone(),
        None => which::which("sclang")
            .map_err(|_| anyhow!("sclang not found on PATH; set --sclang-path"))?
            .display()
            .to_string(),
    };

    match args.mode {
        Mode::Probe => {
            // For now, just run `sclang -v` to confirm availability.
            let output = Command::new(&sclang)
                .arg("-v")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .with_context(|| format!("failed to execute {} -v", sclang))?;

            if !output.status.success() {
                return Err(anyhow!(
                    "sclang probe failed (exit {}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            // Emit a simple JSON probe result to stdout to support a "Check setup" command.
            let probe = serde_json::json!({
                "ok": true,
                "sclang": {
                    "path": sclang,
                    "version": String::from_utf8_lossy(&output.stdout).trim()
                },
                "note": "use --mode lsp to start the LanguageServer bridge"
            });
            println!("{}", probe.to_string());
            Ok(())
        }
        Mode::Lsp => run_lsp_bridge(&sclang, &args),
    }
}

fn run_lsp_bridge(sclang: &str, args: &Args) -> Result<()> {
    // Log version at startup to confirm which binary is running
    eprintln!(
        "[sc_launcher] v{} starting LSP bridge (pid={})",
        env!("CARGO_PKG_VERSION"),
        std::process::id()
    );

    let quark_ok = ensure_quark_present();
    if !quark_ok {
        eprintln!("[sc_launcher] warning: LanguageServer.quark not found in downloaded-quarks; install it via SuperCollider's Quarks GUI or `Quarks.install(\"LanguageServer\");`");
    }

    let ports = allocate_udp_ports().context("failed to reserve UDP ports for LSP bridge")?;
    let shutdown = Arc::new(AtomicBool::new(false));

    let mut command = Command::new(sclang);
    command
        .arg("--daemon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(conf) = args.conf_yaml_path.as_ref() {
        command.arg("--yaml-config").arg(conf);
    }

    // Environment for LanguageServer.quark stdio bridge
    command.env("SCLANG_LSP_ENABLE", "1");
    command.env("SCLANG_LSP_CLIENTPORT", ports.client_port.to_string());
    command.env("SCLANG_LSP_SERVERPORT", ports.server_port.to_string());
    if let Some(level) = args.log_level.as_ref() {
        command.env("SCLANG_LSP_LOGLEVEL", level);
    }

    // Prefer vendored LanguageServer.quark if present (added as a submodule).
    // Prefer vendored LanguageServer.quark if present (added as a submodule).
    // Also try the current working directory (helps when launched directly from repo root).
    let vendored_path = find_vendored_quark_path().or_else(|| {
        std::env::current_dir().ok().and_then(|cwd| {
            let candidate = cwd.join("server/quark/LanguageServer.quark");
            if candidate.exists() {
                Some(candidate.display().to_string())
            } else {
                None
            }
        })
    });

    if let Some(vendor_path) = vendored_path {
        eprintln!("[sc_launcher] including vendored LanguageServer.quark at {}", vendor_path);
        command.arg("--include-path").arg(&vendor_path);

        for installed in installed_quark_paths() {
            eprintln!(
                "[sc_launcher] excluding installed LanguageServer.quark at {}",
                installed.display()
            );
            command
                .arg("--exclude-path")
                .arg(installed.display().to_string());
        }

        // Exclude the built-in ScIDE Document class so the vendored LSPDocument takes precedence.
        // The vendored quark provides its own Document class in scide_vscode/ that properly
        // delegates to LSPDocument for LSP-based document management.
        if let Some(scide_path) = find_scide_scqt_path(sclang) {
            eprintln!(
                "[sc_launcher] excluding built-in scide_scqt at {}",
                scide_path
            );
            command.arg("--exclude-path").arg(scide_path);
        }
    }

    eprintln!(
        "[sc_launcher] spawning sclang (client={}, server={}, log_level={})",
        ports.client_port,
        ports.server_port,
        args.log_level
            .as_deref()
            .unwrap_or("error (LanguageServer default)")
    );

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn sclang at {}", sclang))?;

    // Wait for LSP READY signal from sclang stdout before pumping stdin to UDP
    let (ready_tx, ready_rx) = mpsc::channel();
    let stdout_handle = child
        .stdout
        .take()
        .map(|stream| log_child_stream("sclang stdout", stream, Some(ready_tx.clone())));
    let stderr_handle = child
        .stderr
        .take()
        .map(|stream| log_child_stream("sclang stderr", stream, None));

    let udp_sender = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .context("failed to bind UDP sender socket")?;
    udp_sender
        .connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, ports.client_port))
        .context("failed to connect UDP sender socket")?;

    let udp_receiver = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, ports.server_port))
        .with_context(|| {
        format!(
            "failed to bind UDP receiver socket on port {}",
            ports.server_port
        )
    })?;
    udp_receiver
        .set_read_timeout(Some(Duration::from_millis(200)))
        .context("failed to set UDP receiver timeout")?;

    let (stdin_done_tx, stdin_done_rx) = mpsc::channel();

    // Track request IDs that we've already responded to from the launcher.
    // This prevents sclang's duplicate responses from overwriting ours.
    // We use a String representation of the JSON id for simpler hashing.
    let responded_ids: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // Start the stdin bridge IMMEDIATELY to capture the initialize request from Zed.
    // The bridge will buffer messages until sclang is ready.
    eprintln!("[sc_launcher] about to spawn stdin_bridge thread");
    let _ = std::io::stderr().flush();
    let sclang_ready = Arc::new(AtomicBool::new(false));
    let stdin_bridge = {
        let udp = udp_sender
            .try_clone()
            .context("failed to clone UDP sender socket")?;
        let shutdown = shutdown.clone();
        let done_tx = stdin_done_tx.clone();
        let ready_flag = sclang_ready.clone();
        let responded = responded_ids.clone();
        eprintln!("[sc_launcher] spawning stdin->udp thread NOW");
        let _ = std::io::stderr().flush();
        let handle = thread::Builder::new()
            .name("stdin->udp".into())
            .spawn(move || pump_stdin_to_udp(udp, shutdown, done_tx, ready_flag, responded))
            .context("failed to spawn stdin->udp bridge thread")?;
        eprintln!("[sc_launcher] stdin->udp thread spawned successfully");
        let _ = std::io::stderr().flush();
        handle
    };

    // Start the UDP->stdout bridge BEFORE signaling ready, so we don't miss the initialize response
    let stdout_bridge = {
        let udp = udp_receiver;
        let shutdown = shutdown.clone();
        let responded = responded_ids.clone();
        thread::Builder::new()
            .name("udp->stdout".into())
            .spawn(move || pump_udp_to_stdout(udp, shutdown, responded))
            .context("failed to spawn udp->stdout bridge thread")?
    };

    // Wait for sclang to report LSP READY, then signal the stdin bridge
    let mut waited_ms = 0u64;
    let max_wait_ms = 60_000u64; // 60s
    loop {
        if let Ok(()) = ready_rx.try_recv() {
            eprintln!("[sc_launcher] detected 'LSP READY' from sclang");
            sclang_ready.store(true, Ordering::SeqCst);
            break;
        }
        if waited_ms >= max_wait_ms {
            eprintln!(
                "[sc_launcher] timed out waiting for 'LSP READY' ({}s); proceeding anyway",
                max_wait_ms / 1000
            );
            sclang_ready.store(true, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
        waited_ms += 50;
    }
    let mut stdin_closed = false;

    // Start HTTP server for eval requests
    let http_bridge = {
        let udp = udp_sender
            .try_clone()
            .context("failed to clone UDP sender for HTTP server")?;
        let shutdown = shutdown.clone();
        let port = args.http_port;
        thread::Builder::new()
            .name("http-server".into())
            .spawn(move || run_http_server(port, udp, shutdown))
            .context("failed to spawn HTTP server thread")?
    };

    let status = loop {
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                break Ok(exit_status);
            }
            Ok(None) => {}
            Err(err) => {
                break Err(anyhow!("failed to poll sclang status: {err}"));
            }
        }

        if stdin_done_rx.try_recv().is_ok() {
            stdin_closed = true;
            let exit_status = graceful_shutdown_child(&mut child, &udp_sender, Duration::from_secs(5))
                .context("failed to shut down sclang after stdin closed")?;
            break Ok(exit_status);
        }

        thread::sleep(Duration::from_millis(50));
    }?;

    shutdown.store(true, Ordering::SeqCst);

    let _ = stdin_bridge.join();
    let _ = stdout_bridge.join();
    let _ = http_bridge.join();
    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    if status.success() {
        Ok(())
    } else if stdin_closed {
        eprintln!(
            "[sc_launcher] sclang exited after stdin closed ({})",
            status
        );
        Ok(())
    } else {
        Err(anyhow!("sclang exited with status {}", status))
    }
}

fn find_vendored_quark_path() -> Option<String> {
    // sc_launcher typically lives under <repo>/server/launcher/target/<profile>/sc_launcher
    // Walk up to the repo root and check server/quark/LanguageServer.quark.
    // Also check the current working directory (useful when launched directly from the repo).
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(mut exe) = std::env::current_exe() {
        // ascend: .../server/launcher/target/debug/sc_launcher -> .../server/launcher/target/debug
        exe.pop();
        // -> .../server/launcher/target
        exe.pop();
        // -> .../server/launcher
        exe.pop();
        // -> .../server
        exe.pop();
        // -> <repo>
        exe.pop();
        let mut candidate = exe.clone();
        candidate.push("server/quark/LanguageServer.quark");
        candidates.push(candidate);
    }

    // Check CWD as a fallback (helpful in dev runs where current_exe path is unexpected).
    if let Ok(cwd) = std::env::current_dir() {
        let mut candidate = cwd.clone();
        candidate.push("server/quark/LanguageServer.quark");
        candidates.push(candidate);
    }

    for candidate in candidates {
        if candidate.exists() {
            eprintln!(
                "[sc_launcher] including vendored LanguageServer.quark at {}",
                candidate.display()
            );
            return Some(candidate.display().to_string());
        }
    }

    eprintln!("[sc_launcher] no vendored LanguageServer.quark found in expected locations");
    None
}

fn log_child_stream<R>(
    label: &'static str,
    stream: R,
    ready_signal: Option<mpsc::Sender<()>>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    let verbose = verbose_logging_enabled();
    thread::Builder::new()
        .name(format!("{label}-reader"))
        .spawn(move || {
            // Open post window log file for user-visible output
            let post_log_path = log_dir().join("sclang_post.log");
            let mut post_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&post_log_path)
                .ok();

            if post_file.is_some() && label == "sclang stdout" {
                eprintln!("[sc_launcher] sclang output → {}", post_log_path.display());
            }

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                if !trimmed.is_empty() {
                    if verbose {
                        eprintln!("[{label}] {trimmed}");
                    }

                    // Write stdout to post window log file (filter out verbose LSP debug messages)
                    if let Some(ref mut f) = post_file {
                        // Skip LSP internal protocol messages - users don't need to see these
                        let skip_patterns = [
                            "[LANGUAGESERVER.QUARK] Message received:",
                            "[LANGUAGESERVER.QUARK] Expecting",
                            "[LANGUAGESERVER.QUARK] Found method provider:",
                            "[LANGUAGESERVER.QUARK] Handling:",
                            "[LANGUAGESERVER.QUARK] Checking for client capability",
                            "[LANGUAGESERVER.QUARK] Responding with:",
                            "[LANGUAGESERVER.QUARK] Creating LSP document",
                            "[LANGUAGESERVER.QUARK] Handling a follow-up",
                            "[LANGUAGESERVER.QUARK] client options:",
                            "[LANGUAGESERVER.QUARK] No provider found for method:",
                            "[LANGUAGESERVER.QUARK] Registering provider:",
                            "[LANGUAGESERVER.QUARK] Adding server capability",
                            "[LANGUAGESERVER.QUARK] writing options into key",
                            "[LANGUAGESERVER.QUARK] Adding provider for method",
                            "[LANGUAGESERVER.QUARK] Server capabilities are:",
                            "[LANGUAGESERVER.QUARK] Overwriting provider",
                            "[LANGUAGESERVER.QUARK] initializing",
                            "Deferred(",
                            "{\"jsonrpc\":",
                            "{\"id\":",
                            "{\"method\":",
                            "Dictionary[",
                            "...etc...",
                        ];

                        // Also skip lines that are SC data structure continuations (start with whitespace or special patterns)
                        let is_data_continuation = trimmed.starts_with(' ')
                            || trimmed.starts_with('\t')
                            || (trimmed.starts_with('(') && trimmed.contains("->"))
                            || trimmed.starts_with(", '");

                        let should_skip = skip_patterns.iter().any(|pat| trimmed.contains(pat))
                            || is_data_continuation;
                        if !should_skip {
                            let _ = writeln!(f, "{}", trimmed);
                        }
                    }

                    if let Some(tx) = &ready_signal {
                        if label == "sclang stdout" && trimmed.contains("***LSP READY***") {
                            let _ = tx.send(());
                        }
                    }
                }
                line.clear();
            }
        })
        .expect("failed to spawn child log thread")
}

struct Ports {
    client_port: u16,
    server_port: u16,
}

fn allocate_udp_ports() -> Result<Ports> {
    let client_socket =
        UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).context("bind client port")?;
    let client_port = client_socket.local_addr()?.port();

    let server_socket =
        UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).context("bind server port")?;
    let server_port = server_socket.local_addr()?.port();

    drop(client_socket);
    drop(server_socket);

    Ok(Ports {
        client_port,
        server_port,
    })
}

/// Create an LSP initialize response with server capabilities.
/// This is sent immediately by the launcher so Zed doesn't timeout waiting for sclang.
fn create_initialize_response(id: JsonValue) -> JsonValue {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "capabilities": {
                "textDocumentSync": {
                    "openClose": true,
                    "change": 2,  // Incremental
                    "save": true
                },
                "completionProvider": {
                    "triggerCharacters": [".", "(", "~"],
                    "resolveProvider": false,
                    "completionItem": {
                        "labelDetailsSupport": true
                    }
                },
                "signatureHelpProvider": {
                    "triggerCharacters": ["("],
                    "retriggerCharacters": [","]
                },
                "hoverProvider": true,
                "definitionProvider": true,
                "declarationProvider": true,
                "implementationProvider": true,
                "referencesProvider": true,
                "selectionRangeProvider": {},
                "foldingRangeProvider": {},
                "codeLensProvider": {},
                "codeActionProvider": {
                    "codeActionKinds": ["source"]
                },
                "workspaceSymbolProvider": {},
                "executeCommandProvider": {
                    "commands": [
                        "supercollider.eval",
                        "supercollider.evaluateSelection",
                        "supercollider.internal.bootServer",
                        "supercollider.internal.rebootServer",
                        "supercollider.internal.quitServer",
                        "supercollider.internal.recompile",
                        "supercollider.internal.cmdPeriod"
                    ]
                }
            },
            "serverInfo": {
                "name": "sclang:LSPConnection",
                "version": "0.1"
            }
        }
    })
}

fn graceful_shutdown_child(
    child: &mut std::process::Child,
    udp_socket: &UdpSocket,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    eprintln!(
        "[sc_launcher] stdin closed; requesting LSP shutdown (timeout {:?})",
        timeout
    );
    request_lsp_shutdown(udp_socket);

    if let Some(stdin) = child.stdin.take() {
        drop(stdin);
    }

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(err) => return Err(anyhow!("failed to poll sclang status: {err}")),
        }
        thread::sleep(Duration::from_millis(100));
    }

    #[cfg(unix)]
    {
        let pid = child.id();
        eprintln!("[sc_launcher] sending SIGTERM to sclang pid {}", pid);
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();

        let term_start = std::time::Instant::now();
        while term_start.elapsed() < Duration::from_secs(2) {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {}
                Err(err) => {
                    return Err(anyhow!(
                        "failed to poll sclang status after SIGTERM: {err}"
                    ))
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    eprintln!("[sc_launcher] forcing sclang shutdown with kill");
    child
        .kill()
        .context("failed to kill sclang process after shutdown request")?;
    child
        .wait()
        .context("failed to wait for sclang after forced shutdown")
}

fn pump_stdin_to_udp(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
    done_tx: mpsc::Sender<()>,
    sclang_ready: Arc<AtomicBool>,
    responded_ids: Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    let verbose = verbose_logging_enabled();
    // Cache the most recent didOpen/didChange to resend after providers register.
    let cached_did_open: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let cached_did_change: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

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
        use std::io::Write;
        let _ = writeln!(
            f,
            "\n=== pump_stdin_to_udp ENTERED at {:?} ===",
            std::time::SystemTime::now()
        );
    }

    if verbose {
        eprintln!("[sc_launcher] pump_stdin_to_udp: ENTERED FUNCTION");
        let _ = std::io::stderr().flush();
    }

    let stdin = io::stdin();
    if verbose {
        eprintln!("[sc_launcher] pump_stdin_to_udp: got stdin handle");
        let _ = std::io::stderr().flush();
    }

    let mut reader = BufReader::new(stdin.lock());
    if verbose {
        eprintln!("[sc_launcher] pump_stdin_to_udp: created BufReader with stdin lock");
        let _ = std::io::stderr().flush();
    }

    // Use a channel to queue messages for sending (allows separate flush thread)
    let (msg_tx, msg_rx) = mpsc::channel::<Vec<u8>>();

    // Spawn a sender thread that buffers until sclang is ready, then sends
    let sender_socket = socket
        .try_clone()
        .context("failed to clone socket for sender")?;
    let sender_ready = sclang_ready.clone();
    let sender_shutdown = shutdown.clone();
    let resend_did_open = cached_did_open.clone();
    let resend_did_change = cached_did_change.clone();
    if verbose {
        eprintln!("[sc_launcher] pump_stdin_to_udp: about to spawn sender thread");
        let _ = std::io::stderr().flush();
    }
    let sender_thread = thread::Builder::new()
        .name("stdin-sender".into())
        .spawn(move || {
            let sender_start = std::time::Instant::now();
            if verbose {
                eprintln!("[sc_launcher] stdin-sender thread started at t=0ms");
                let _ = std::io::stderr().flush();
            }
            let mut pending_messages: Vec<Vec<u8>> = Vec::new();
            let mut ready_signaled = false;

            loop {
                // Check for ready signal
                if !ready_signaled && sender_ready.load(Ordering::SeqCst) {
                    ready_signaled = true;
                    if verbose {
                        eprintln!(
                            "[sc_launcher] sender thread: sclang ready at t={}ms, {} messages buffered",
                            sender_start.elapsed().as_millis(),
                            pending_messages.len()
                        );
                    }
                    // Resend last didOpen/didChange after providers are likely registered.
                    if let Some(open_msg) = resend_did_open.lock().ok().and_then(|m| m.clone()) {
                        if verbose {
                            eprintln!(
                                "[sc_launcher] re-sending cached textDocument/didOpen after sclang ready"
                            );
                        }
                        pending_messages.push(open_msg);
                    }
                    if let Some(change_msg) = resend_did_change.lock().ok().and_then(|m| m.clone())
                    {
                        if verbose {
                            eprintln!(
                                "[sc_launcher] re-sending cached textDocument/didChange after sclang ready"
                            );
                        }
                        pending_messages.push(change_msg);
                    }
                    if !pending_messages.is_empty() {
                        if verbose {
                            eprintln!(
                                "[sc_launcher] sclang ready, flushing {} buffered messages at t={}ms",
                                pending_messages.len(),
                                sender_start.elapsed().as_millis()
                            );
                        }
                        for msg in pending_messages.drain(..) {
                            if let Err(err) = send_with_retry(&sender_socket, &msg) {
                                eprintln!(
                                    "[sc_launcher] failed to send buffered UDP message: {err}"
                                );
                            }
                        }
                        if verbose {
                            eprintln!(
                                "[sc_launcher] finished flushing buffered messages at t={}ms",
                                sender_start.elapsed().as_millis()
                            );
                        }
                    } else {
                        if verbose {
                            eprintln!("[sc_launcher] sclang ready, no buffered messages to flush");
                        }
                    }
                }

                // Try to receive a message (with timeout to allow checking ready flag)
                match msg_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(message) => {
                        if ready_signaled {
                            if let Err(err) = send_with_retry(&sender_socket, &message) {
                                eprintln!("[sc_launcher] failed to send UDP message: {err}");
                            }
                        } else {
                            pending_messages.push(message);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Continue checking ready flag
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Reader thread closed, flush any remaining and exit
                        if !pending_messages.is_empty() && ready_signaled {
                            for msg in pending_messages.drain(..) {
                                let _ = send_with_retry(&sender_socket, &msg);
                            }
                        }
                        break;
                    }
                }

                if sender_shutdown.load(Ordering::SeqCst) {
                    break;
                }
            }
        })?;

    eprintln!("[sc_launcher] stdin reader: starting main loop");
    let _ = std::io::stderr().flush();

    if let Some(ref mut f) = stdin_log {
        use std::io::Write;
        let _ = writeln!(
            f,
            "stdin reader: starting main loop at {:?}",
            std::time::SystemTime::now()
        );
    }
    let mut msg_count = 0u64;

    // Main loop: read from stdin and queue messages to sender thread
    while !shutdown.load(Ordering::SeqCst) {
        match read_lsp_message(&mut reader) {
            Ok(Some(message)) => {
                msg_count += 1;
                if verbose {
                    eprintln!(
                        "[sc_launcher] stdin reader: got message {} bytes",
                        message.len()
                    );
                }
                // Log to file
                if let Some(ref mut f) = stdin_log {
                    use std::io::Write;
                    let preview = String::from_utf8_lossy(&message[..message.len().min(500)]);
                    let _ = writeln!(
                        f,
                        "MSG#{} ({} bytes): {}",
                        msg_count,
                        message.len(),
                        preview
                    );
                }
                // Log incoming LSP method for debugging and handle initialize specially
                let is_buffered = !sclang_ready.load(Ordering::SeqCst);
                let should_forward = true;

                if let Ok(body_str) = std::str::from_utf8(&message) {
                    if let Some(body_start) = body_str.find("\r\n\r\n") {
                        let body = &body_str[body_start + 4..];
                        if let Ok(json) = serde_json::from_str::<JsonValue>(body) {
                            if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                                if verbose {
                                    eprintln!(
                                        "[sc_launcher] << LSP request: {} (id={:?}) size={} {}",
                                        method,
                                        json.get("id"),
                                        message.len(),
                                        if is_buffered { "[BUFFERED]" } else { "" }
                                    );
                                }
                                // Cache last didOpen/didChange so we can replay after sclang is ready.
                                if method == "textDocument/didOpen" {
                                    if let Ok(mut slot) = cached_did_open.lock() {
                                        *slot = Some(message.clone());
                                    }
                                } else if method == "textDocument/didChange" {
                                    if let Ok(mut slot) = cached_did_change.lock() {
                                        *slot = Some(message.clone());
                                    }
                                }

                                // Handle initialize request IMMEDIATELY from the launcher
                                // We can't wait for sclang because Zed expects a fast response
                                if method == "initialize" {
                                    if let Some(id) = json.get("id") {
                                        eprintln!("[sc_launcher] INTERCEPTING initialize request - responding immediately");
                                        let response = create_initialize_response(id.clone());
                                        let response_json =
                                            serde_json::to_string(&response).unwrap();
                                        let response_msg = format!(
                                            "Content-Length: {}\r\n\r\n{}",
                                            response_json.len(),
                                            response_json
                                        );

                                        // Write directly to stdout
                                        let mut stdout = io::stdout();
                                        if let Err(e) = stdout.write_all(response_msg.as_bytes()) {
                                            eprintln!("[sc_launcher] failed to write initialize response: {}", e);
                                        }
                                        if let Err(e) = stdout.flush() {
                                            eprintln!("[sc_launcher] failed to flush initialize response: {}", e);
                                        }
                                        if verbose {
                                            eprintln!("[sc_launcher] sent initialize response to Zed");
                                        }

                                        // Log to file
                                        if let Some(ref mut f) = stdin_log {
                                            use std::io::Write;
                                            let _ = writeln!(
                                                f,
                                                ">>> RESPONDED TO INITIALIZE: {}",
                                                response_json
                                            );
                                        }

                                        // Record that we've already responded to this request ID
                                        // so we can suppress sclang's duplicate response
                                        let id_str = id.to_string();
                                        if let Ok(mut set) = responded_ids.lock() {
                                            set.insert(id_str.clone());
                                            if verbose {
                                                eprintln!("[sc_launcher] recorded responded id={} for suppression", id_str);
                                            }
                                        }

                                        // Still forward to sclang so it can set up its state
                                        // but we've already responded to Zed
                                    }
                                }
                            }
                        }
                    }
                }

                // Queue message for sender thread (forward to sclang)
                if should_forward {
                    if msg_tx.send(message).is_err() {
                        eprintln!("[sc_launcher] sender thread closed unexpectedly");
                        break;
                    }
                }
            }
            Ok(None) => {
                // graceful shutdown (stdin closed)
                let _ = done_tx.send(());
                break;
            }
            Err(err) => {
                eprintln!("[sc_launcher] error reading from stdin: {err}");
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

fn pump_udp_to_stdout(
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
    responded_ids: Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    let verbose = verbose_logging_enabled();
    let start = std::time::Instant::now();
    if verbose {
        eprintln!(
            "[sc_launcher] UDP->stdout bridge STARTED at t=0ms, listening on {:?}",
            socket.local_addr()
        );
        // Force flush stderr immediately so this message appears
        let _ = std::io::stderr().flush();
    }
    let mut dgram_buf = vec![0u8; 64 * 1024];
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

    let mut total_packets = 0u64;
    while !shutdown.load(Ordering::SeqCst) {
        match socket.recv(&mut dgram_buf) {
            Ok(size) => {
                if size == 0 {
                    continue;
                }
                total_packets += 1;
                if verbose {
                    eprintln!(
                        "[sc_launcher] UDP packet #{} received: {} bytes at t={}ms",
                        total_packets,
                        size,
                        start.elapsed().as_millis()
                    );
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
                        let mut patched = false;
                        if let Ok(mut value) = serde_json::from_slice::<JsonValue>(&body) {
                            if value.get("jsonrpc").is_none() {
                                if let JsonValue::Object(ref mut map) = value {
                                    map.insert(
                                        "jsonrpc".to_string(),
                                        JsonValue::String("2.0".to_string()),
                                    );
                                    if let Ok(vec) = serde_json::to_vec(&value) {
                                        body = vec;
                                        patched = true;
                                    }
                                }
                            }
                        }

                        if patched {
                            if verbose {
                                eprintln!(
                                    "[sc_launcher] patched missing jsonrpc field in server message"
                                );
                            }
                        }

                        // Check if this is a response to a request we've already handled
                        // (e.g., initialize response from sclang when we already responded)
                        let mut should_suppress = false;
                        if let Ok(json) = serde_json::from_slice::<JsonValue>(&body) {
                            if let Some(id) = json.get("id") {
                                let id_str = id.to_string();
                                if let Ok(set) = responded_ids.lock() {
                                    if set.contains(&id_str) {
                                        should_suppress = true;
                                        if verbose {
                                            eprintln!(
                                                "[sc_launcher] SUPPRESSING duplicate response for id={} (already responded from launcher)",
                                                id_str
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        if should_suppress {
                            // Skip writing this response to stdout
                            continue 'outer;
                        }

                        // Write exactly one LSP message to stdout, potentially patched.
                        let header = format!("Content-Length: {}\r\n\r\n", body.len());
                        if let Err(err) = stdout.write_all(header.as_bytes()) {
                            eprintln!("[sc_launcher] failed to write header: {err}");
                            break;
                        }
                        if let Err(err) = stdout.write_all(&body) {
                            eprintln!("[sc_launcher] failed to write LSP body: {err}");
                            break;
                        }
                        if let Err(err) = stdout.flush() {
                            eprintln!("[sc_launcher] failed to flush stdout: {err}");
                            break;
                        }
                        if verbose {
                            let preview = String::from_utf8_lossy(&body[..body.len().min(200)]);
                            eprintln!(
                                "[sc_launcher] >> {} bytes to stdout (first 200): {}",
                                body.len(),
                                preview
                            );
                            // Extra: log if this looks like an initialize response (has capabilities)
                            if body.len() > 50 {
                                let body_str = String::from_utf8_lossy(&body);
                                if body_str.contains("capabilities") {
                                    eprintln!(
                                        "[sc_launcher] !!! CAPABILITIES DETECTED in response at t={}ms !!!",
                                        start.elapsed().as_millis()
                                    );
                                    eprintln!("[sc_launcher] FULL RESPONSE: {}", body_str);
                                }
                            }
                        }

                        // Log full initialize response for debugging capabilities
                        if verbose {
                            if let Ok(json) = serde_json::from_slice::<JsonValue>(&body) {
                                // Check for capabilities in result (initialize response)
                                if let Some(result) = json.get("result") {
                                    if result.get("capabilities").is_some() {
                                        eprintln!(
                                            "[sc_launcher] *** SERVER CAPABILITIES ***:\n{}",
                                            serde_json::to_string_pretty(
                                                result.get("capabilities").unwrap()
                                            )
                                            .unwrap_or_default()
                                        );
                                    }
                                }
                                // Log all response ids for debugging
                                if let Some(id) = json.get("id") {
                                    eprintln!(
                                        "[sc_launcher] >> response id={} type={}",
                                        id,
                                        if id.is_i64() {
                                            "int"
                                        } else if id.is_string() {
                                            "str"
                                        } else {
                                            "?"
                                        }
                                    );
                                }
                            }
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
                eprintln!("[sc_launcher] UDP receive error: {err}");
                break;
            }
        }
    }

    shutdown.store(true, Ordering::SeqCst);
    Ok(())
}

fn send_with_retry(socket: &UdpSocket, message: &[u8]) -> io::Result<()> {
    use std::io::ErrorKind;
    const RETRY_SLEEP_MS: u64 = 50;
    const MAX_RETRY_MS: u64 = 90_000;
    // Match LanguageServer.quark's maxSize for UDP chunking
    const MAX_CHUNK_SIZE: usize = 6000;
    let verbose = verbose_logging_enabled();

    let mut attempts = 0usize;
    let max_attempts = (MAX_RETRY_MS / RETRY_SLEEP_MS) as usize;

    // Log what we're sending (extract method if possible)
    if verbose {
        if let Ok(msg_str) = std::str::from_utf8(message) {
            if let Some(body_start) = msg_str.find("\r\n\r\n") {
                let body = &msg_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<JsonValue>(body) {
                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                        eprintln!(
                            "[sc_launcher] >>> SENDING to sclang: method={} id={:?} size={}",
                            method,
                            json.get("id"),
                            message.len()
                        );
                    }
                }
            }
        }
    }

    // If message fits in one packet, send directly
    if message.len() <= MAX_CHUNK_SIZE {
        loop {
            match socket.send(message) {
                Ok(bytes) if bytes == message.len() => return Ok(()),
                Ok(_) => {
                    return Err(io::Error::new(
                        ErrorKind::Other,
                        "partial UDP send (wrote fewer bytes than expected)",
                    ))
                }
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    if verbose {
                        if attempts == 0 || attempts % 40 == 0 {
                            eprintln!(
                                "[sc_launcher] Connection refused sending to sclang (attempt {}): {err}",
                                attempts + 1
                            );
                        }
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
                    std::thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                    attempts += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    // Large message: chunk it like LanguageServer.quark does
    if verbose {
        eprintln!(
            "[sc_launcher] chunking large message ({} bytes) into {} chunks",
            message.len(),
            (message.len() + MAX_CHUNK_SIZE - 1) / MAX_CHUNK_SIZE
        );
    }

    let mut offset = 0;
    while offset < message.len() {
        let end = (offset + MAX_CHUNK_SIZE).min(message.len());
        let chunk = &message[offset..end];

        loop {
            match socket.send(chunk) {
                Ok(bytes) if bytes == chunk.len() => break,
                Ok(_) => {
                    return Err(io::Error::new(
                        ErrorKind::Other,
                        "partial UDP send on chunk",
                    ))
                }
                Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
                    if verbose {
                        if attempts == 0 || attempts % 40 == 0 {
                            eprintln!(
                                "[sc_launcher] Connection refused sending chunk (attempt {}): {err}",
                                attempts + 1
                            );
                        }
                    }
                    if attempts >= max_attempts {
                        return Err(io::Error::new(
                            ErrorKind::ConnectionRefused,
                            format!("connection refused after {} retries: {err}", attempts + 1),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(RETRY_SLEEP_MS));
                    attempts += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
        offset = end;
        // Small delay between chunks to avoid overwhelming the receiver
        std::thread::sleep(Duration::from_micros(100));
    }

    Ok(())
}

fn read_lsp_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
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

fn installed_quark_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let mut downloaded = std::path::PathBuf::from(&home);
        downloaded
            .push("Library/Application Support/SuperCollider/downloaded-quarks/LanguageServer");
        if downloaded.exists() {
            paths.push(downloaded);
        }

        let mut extensions = std::path::PathBuf::from(home);
        extensions.push("Library/Application Support/SuperCollider/Extensions/LanguageServer");
        if extensions.exists() {
            paths.push(extensions);
        }
    }
    paths
}

fn ensure_quark_present() -> bool {
    !installed_quark_paths().is_empty()
}

/// Global request ID counter for launcher-originated LSP requests.
static NEXT_LSP_REQUEST_ID: AtomicU64 = AtomicU64::new(1_000_000);

fn next_lsp_request_id() -> u64 {
    NEXT_LSP_REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

fn send_lsp_payload(udp_socket: &UdpSocket, payload: &serde_json::Value) -> io::Result<()> {
    let lsp_json =
        serde_json::to_string(payload).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    let lsp_message = format!("Content-Length: {}\r\n\r\n{}", lsp_json.len(), lsp_json);

    udp_socket.send(lsp_message.as_bytes()).map(|_| ())
}

/// Run the HTTP server for eval requests.
/// Accepts POST /eval with code in the body, sends workspace/executeCommand to sclang.
fn run_http_server(port: u16, udp_socket: UdpSocket, shutdown: Arc<AtomicBool>) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = match Server::http(&addr) {
        Ok(s) => s,
        Err(err) => {
            eprintln!(
                "[sc_launcher] failed to start HTTP server on {}: {}",
                addr, err
            );
            return Err(anyhow!("HTTP server bind failed: {}", err));
        }
    };

    eprintln!(
        "[sc_launcher] HTTP eval server listening on http://{}",
        addr
    );

    // Set a timeout so we can check shutdown flag periodically
    server
        .incoming_requests()
        .into_iter()
        .take_while(|_| !shutdown.load(Ordering::SeqCst))
        .for_each(|mut request| {
            let response = handle_http_request(&mut request, &udp_socket);
            if let Err(err) = request.respond(response) {
                eprintln!("[sc_launcher] failed to send HTTP response: {}", err);
            }
        });

    eprintln!("[sc_launcher] HTTP server shutting down");
    Ok(())
}

fn handle_http_request(
    request: &mut tiny_http::Request,
    udp_socket: &UdpSocket,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let url = request.url().to_string();
    let method = request.method().clone();

    // CORS preflight
    if method == Method::Options {
        return Response::from_string("")
            .with_status_code(204)
            .with_header(
                tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                    .unwrap(),
            )
            .with_header(
                tiny_http::Header::from_bytes(
                    &b"Access-Control-Allow-Methods"[..],
                    &b"POST, OPTIONS"[..],
                )
                .unwrap(),
            )
            .with_header(
                tiny_http::Header::from_bytes(
                    &b"Access-Control-Allow-Headers"[..],
                    &b"Content-Type"[..],
                )
                .unwrap(),
            );
    }

    // Health check endpoint
    if url == "/health" && method == Method::Get {
        let body = r#"{"status":"ok"}"#;
        return Response::from_string(body)
            .with_status_code(200)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            );
    }

    // Eval endpoint
    if url == "/eval" && method == Method::Post {
        let mut body = String::new();
        if let Err(err) = request.as_reader().read_to_string(&mut body) {
            let error_body = format!(r#"{{"error":"failed to read body: {}"}}"#, err);
            return Response::from_string(error_body)
                .with_status_code(400)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                );
        }

        // Send workspace/executeCommand to sclang via UDP
        let request_id = next_lsp_request_id();
        let lsp_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "workspace/executeCommand",
            "params": {
                "command": "supercollider.eval",
                "arguments": [body]
            }
        });

        match send_lsp_payload(udp_socket, &lsp_request) {
            Ok(_) => {
                eprintln!(
                    "[sc_launcher] HTTP /eval sent {} bytes to sclang (id={})",
                    body.len(),
                    request_id
                );
                // We don't wait for the LSP response - fire and forget for now
                // The result will be posted to sclang's post window
                let response_body = format!(
                    r#"{{"status":"sent","request_id":{},"code_length":{}}}"#,
                    request_id,
                    body.len()
                );
                Response::from_string(response_body)
                    .with_status_code(202)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Origin"[..],
                            &b"*"[..],
                        )
                        .unwrap(),
                    )
            }
            Err(err) => {
                eprintln!("[sc_launcher] HTTP /eval failed to send UDP: {}", err);
                let error_body = format!(r#"{{"error":"failed to send to sclang: {}"}}"#, err);
                Response::from_string(error_body)
                    .with_status_code(502)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .unwrap(),
                    )
            }
        }
    // Stop endpoint - CmdPeriod.run
    } else if url == "/stop" && method == Method::Post {
        send_command(udp_socket, "supercollider.internal.cmdPeriod", &[])
    // Boot endpoint - Server.default.boot
    } else if url == "/boot" && method == Method::Post {
        send_command(udp_socket, "supercollider.internal.bootServer", &[])
    // Recompile endpoint - thisProcess.recompile
    } else if url == "/recompile" && method == Method::Post {
        send_command(udp_socket, "supercollider.internal.recompile", &[])
    // Quit server endpoint - Server.default.quit
    } else if url == "/quit" && method == Method::Post {
        send_command(udp_socket, "supercollider.internal.quitServer", &[])
    } else {
        let body = r#"{"error":"not found","endpoints":["/eval","/health","/stop","/boot","/recompile","/quit"]}"#;
        Response::from_string(body)
            .with_status_code(404)
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            )
    }
}

/// Send a workspace/executeCommand to sclang and return an HTTP response.
fn send_command(
    udp_socket: &UdpSocket,
    command: &str,
    arguments: &[&str],
) -> Response<std::io::Cursor<Vec<u8>>> {
    let request_id = next_lsp_request_id();
    let args: Vec<serde_json::Value> = arguments.iter().map(|s| serde_json::json!(s)).collect();
    let lsp_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "workspace/executeCommand",
        "params": {
            "command": command,
            "arguments": args
        }
    });

    match send_lsp_payload(udp_socket, &lsp_request) {
        Ok(_) => {
            eprintln!(
                "[sc_launcher] HTTP /{} sent command {} (id={})",
                command.split('.').last().unwrap_or(command),
                command,
                request_id
            );
            let response_body = format!(
                r#"{{"status":"sent","command":"{}","request_id":{}}}"#,
                command, request_id
            );
            Response::from_string(response_body)
                .with_status_code(202)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
                .with_header(
                    tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..])
                        .unwrap(),
                )
        }
        Err(err) => {
            eprintln!(
                "[sc_launcher] HTTP /{} failed to send UDP: {}",
                command.split('.').last().unwrap_or(command),
                err
            );
            let error_body = format!(r#"{{"error":"failed to send to sclang: {}"}}"#, err);
            Response::from_string(error_body)
                .with_status_code(502)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
        }
    }
}

fn request_lsp_shutdown(udp_socket: &UdpSocket) {
    let shutdown_id = next_lsp_request_id();
    let shutdown_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": shutdown_id,
        "method": "shutdown",
        "params": {}
    });

    if let Err(err) = send_lsp_payload(udp_socket, &shutdown_request) {
        eprintln!("[sc_launcher] failed to send shutdown request: {}", err);
    }

    let exit_notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit"
    });

    if let Err(err) = send_lsp_payload(udp_socket, &exit_notification) {
        eprintln!("[sc_launcher] failed to send exit notification: {}", err);
    }
}

/// Find the path to the built-in scide_scqt directory containing the ScIDE Document class.
/// This needs to be excluded when using the vendored LanguageServer.quark which provides
/// its own Document class that delegates to LSPDocument.
fn find_scide_scqt_path(sclang_path: &str) -> Option<String> {
    // sclang is typically at:
    //   macOS: /Applications/SuperCollider.app/Contents/MacOS/sclang
    //   Linux: /usr/bin/sclang or similar
    // SCClassLibrary is at:
    //   macOS: /Applications/SuperCollider.app/Contents/Resources/SCClassLibrary
    //   Linux: /usr/share/SuperCollider/SCClassLibrary or similar

    let sclang = std::path::Path::new(sclang_path);

    // Try macOS layout first: sclang -> MacOS -> Contents -> Resources/SCClassLibrary
    if let Some(contents) = sclang.parent().and_then(|p| p.parent()) {
        let scide_path = contents.join("Resources/SCClassLibrary/scide_scqt");
        if scide_path.exists() {
            return Some(scide_path.display().to_string());
        }
    }

    // Try Linux layout: look for SCClassLibrary relative to sclang or in common locations
    let linux_paths = [
        "/usr/share/SuperCollider/SCClassLibrary/scide_scqt",
        "/usr/local/share/SuperCollider/SCClassLibrary/scide_scqt",
    ];
    for path in linux_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}
