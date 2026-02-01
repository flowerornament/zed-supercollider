//! LSP bridge orchestration and coordination.
//!
//! Manages the lifecycle of the sclang process and coordinates the
//! stdin↔UDP↔stdout bridges.

use anyhow::{anyhow, Context, Result};
use fslock::LockFile;
use log::{debug, error, info, warn};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::bridge::{
    create_lsp_notification, create_lsp_request, next_lsp_request_id, pump_stdin_to_udp,
    pump_udp_to_stdout, RequestId,
};
use crate::constants::*;
use crate::http;
use crate::logging::{log_child_stream, log_dir};
use crate::process::{
    cleanup_orphaned_processes, ensure_quark_present, find_scide_scqt_path,
    find_vendored_quark_path, installed_quark_paths, make_sclang_command, remove_pid_file,
    write_pid_file,
};
use crate::Args;

// ============================================================================
// State Types
// ============================================================================

/// UDP port pair for LSP communication.
pub struct Ports {
    pub client_port: u16,
    pub server_port: u16,
}

/// State for tracking the sclang child process.
pub struct ChildState {
    pub pid: u32,
    pub run_token: u64,
    pub owned: AtomicBool,
}

/// RAII guard for the IS_RUNNING flag.
pub struct RunningGuard {
    pub run_token: u64,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        IS_RUNNING.store(false, Ordering::SeqCst);
        debug!("run token {}: cleared running guard", self.run_token);
    }
}

// ============================================================================
// Global State
// ============================================================================

/// Monotonic token to tag each launcher run for log disambiguation.
pub static RUN_TOKEN: AtomicU64 = AtomicU64::new(1);

/// Simple guard to prevent multiple concurrent sclang spawns from this process.
pub static IS_RUNNING: AtomicBool = AtomicBool::new(false);

// ============================================================================
// Port Allocation
// ============================================================================

/// Allocate two UDP ports for LSP communication.
/// Returns a Ports struct with client and server port numbers.
pub fn allocate_udp_ports() -> Result<Ports> {
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

/// Release ownership of the child process state.
pub fn release_child_state(state: &Arc<Mutex<Option<ChildState>>>) {
    if let Ok(mut slot) = state.lock() {
        if let Some(child) = slot.take() {
            child.owned.store(false, Ordering::SeqCst);
            debug!(
                "run token {}: released tracked sclang pid {}",
                child.run_token, child.pid
            );
        }
    }
}

// ============================================================================
// Shutdown Handling
// ============================================================================

/// Send LSP shutdown and exit requests, returning Result for retry handling.
fn request_lsp_shutdown_with_result(udp_socket: &UdpSocket) -> std::io::Result<()> {
    let shutdown_id = next_lsp_request_id();
    let shutdown_request = create_lsp_request(shutdown_id, "shutdown", serde_json::json!({}));
    http::send_lsp_payload(udp_socket, &shutdown_request)?;

    let exit_notification = create_lsp_notification("exit", serde_json::json!({}));
    http::send_lsp_payload(udp_socket, &exit_notification)?;

    Ok(())
}

/// Gracefully shutdown the sclang child process.
/// Sends LSP shutdown/exit, waits for graceful exit, then uses signals if needed.
pub fn graceful_shutdown_child(
    child: &mut std::process::Child,
    udp_socket: &UdpSocket,
    timeout: std::time::Duration,
    run_token: u64,
) -> Result<std::process::ExitStatus> {
    let pid = child.id();
    debug!(
        "run token {}: initiating graceful shutdown for pid {} (timeout {:?})",
        run_token, pid, timeout
    );

    // Attempt LSP shutdown with retries
    let mut shutdown_sent = false;
    for attempt in 1..=SHUTDOWN_RETRY_ATTEMPTS {
        match request_lsp_shutdown_with_result(udp_socket) {
            Ok(_) => {
                shutdown_sent = true;
                debug!("LSP shutdown/exit sent successfully (attempt {})", attempt);
                break;
            }
            Err(e) => {
                debug!("LSP shutdown attempt {} failed: {}", attempt, e);
                if attempt < SHUTDOWN_RETRY_ATTEMPTS {
                    thread::sleep(millis_to_duration(SHUTDOWN_RETRY_DELAY_MS));
                }
            }
        }
    }

    if !shutdown_sent {
        warn!("could not send LSP shutdown, will rely on SIGTERM");
    }

    // Close sclang's stdin to signal EOF
    if let Some(stdin) = child.stdin.take() {
        drop(stdin);
    }

    // Wait for graceful exit
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => {
                debug!("sclang exited gracefully with {}", status);
                return Ok(status);
            }
            Ok(None) => {}
            Err(err) => return Err(anyhow!("failed to poll sclang status: {err}")),
        }
        thread::sleep(millis_to_duration(SHUTDOWN_POLL_MS));
    }

    #[cfg(unix)]
    {
        use crate::process::signal;

        let pid = child.id();
        debug!(
            "run token {}: sending SIGTERM to sclang pid {}",
            run_token, pid
        );
        let _ = signal::send_sigterm(pid);

        let term_start = std::time::Instant::now();
        while term_start.elapsed() < SIGTERM_GRACE_PERIOD {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {}
                Err(err) => {
                    return Err(anyhow!(
                        "run token {}: failed to poll sclang status after SIGTERM: {err}",
                        run_token
                    ))
                }
            }
            thread::sleep(millis_to_duration(SHUTDOWN_POLL_MS));
        }
    }

    debug!("run token {}: forcing sclang shutdown with kill", run_token);
    child
        .kill()
        .context("failed to kill sclang process after shutdown request")?;
    child
        .wait()
        .context("failed to wait for sclang after forced shutdown")
}

// ============================================================================
// Main LSP Bridge
// ============================================================================

/// Run the LSP bridge between Zed and sclang.
/// This is the main entry point for LSP mode.
pub fn run_lsp_bridge(sclang: &str, args: &Args) -> Result<()> {
    let startup_start = Instant::now();

    // Clean up any orphaned sclang processes from previous launcher instances
    cleanup_orphaned_processes();

    // Acquire exclusive lock to ensure single instance.
    // This prevents port conflicts when Zed restarts quickly.
    let lock_path = log_dir().join("sc_launcher.lock");
    let mut lock = LockFile::open(&lock_path)
        .map_err(|e| anyhow!("failed to open lock file {:?}: {}", lock_path, e))?;
    if !lock.try_lock().unwrap_or(false) {
        debug!("waiting for previous instance to release lock...");
        // Block until lock is available (previous instance exiting)
        lock.lock()
            .map_err(|e| anyhow!("failed to acquire lock: {}", e))?;
    }
    // Lock is held for process lifetime - auto-releases on exit

    let run_token = RUN_TOKEN.fetch_add(1, Ordering::SeqCst);
    if IS_RUNNING.swap(true, Ordering::SeqCst) {
        error!(
            "run token {}: launcher already running; refusing second spawn",
            run_token
        );
        return Err(anyhow!(
            "sc_launcher already running (token {}) - refusing duplicate spawn",
            run_token
        ));
    }
    let _run_guard = RunningGuard { run_token };
    // Log version at startup to confirm which binary is running
    info!(
        "v{} starting LSP bridge (pid={}, run={})",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        run_token
    );

    let quark_ok = ensure_quark_present();
    if !quark_ok {
        warn!("LanguageServer.quark not found in downloaded-quarks; install it via SuperCollider's Quarks GUI or `Quarks.install(\"LanguageServer\");`");
    }

    let ports = allocate_udp_ports().context("failed to reserve UDP ports for LSP bridge")?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let child_state: Arc<Mutex<Option<ChildState>>> = Arc::new(Mutex::new(None));

    let mut command = make_sclang_command(sclang);
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
    let vendored_path = find_vendored_quark_path();

    if let Some(vendor_path) = vendored_path {
        debug!("including vendored LanguageServer.quark at {}", vendor_path);
        command.arg("--include-path").arg(&vendor_path);

        for installed in installed_quark_paths() {
            debug!(
                "excluding installed LanguageServer.quark at {}",
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
            debug!("excluding built-in scide_scqt at {}", scide_path);
            command.arg("--exclude-path").arg(scide_path);
        }
    }

    debug!(
        "spawning sclang (client={}, server={}, log_level={})",
        ports.client_port,
        ports.server_port,
        args.log_level
            .as_deref()
            .unwrap_or("error (LanguageServer default)")
    );

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn sclang at {}", sclang))?;

    {
        let pid = child.id();
        let mut slot = child_state.lock().unwrap_or_else(|e| e.into_inner());
        *slot = Some(ChildState {
            pid,
            run_token,
            owned: AtomicBool::new(true),
        });
        debug!("run token {}: spawned sclang pid={}", run_token, pid);
        // Write PID file for safe cleanup by external tools
        if let Err(e) = write_pid_file(std::process::id(), pid) {
            warn!("{}", e);
        }
    }

    // Wait for LSP READY signal from sclang stdout before pumping stdin to UDP
    let (ready_tx, ready_rx) = mpsc::channel();
    // Track ready count for recompile detection (increments each time LSP READY is seen)
    let ready_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let stdout_handle = child.stdout.take().map(|stream| {
        log_child_stream(
            "sclang stdout",
            stream,
            Some(ready_tx.clone()),
            Some(ready_count.clone()),
        )
    });
    let stderr_handle = child
        .stderr
        .take()
        .map(|stream| log_child_stream("sclang stderr", stream, None, None));

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
        .set_read_timeout(Some(millis_to_duration(UDP_READ_TIMEOUT_MS)))
        .context("failed to set UDP receiver timeout")?;

    let (stdin_done_tx, stdin_done_rx) = mpsc::channel();

    // Track request IDs that we've already responded to from the launcher.
    // This prevents sclang's duplicate responses from overwriting ours.
    let responded_ids: Arc<Mutex<HashSet<RequestId>>> = Arc::new(Mutex::new(HashSet::new()));

    // Start the stdin bridge IMMEDIATELY to capture the initialize request from Zed.
    // The bridge will buffer messages until sclang is ready.
    debug!("about to spawn stdin_bridge thread");
    let sclang_ready = Arc::new(AtomicBool::new(false));
    let stdin_bridge = {
        let udp = udp_sender
            .try_clone()
            .context("failed to clone UDP sender socket")?;
        let shutdown = shutdown.clone();
        let done_tx = stdin_done_tx.clone();
        let ready_flag = sclang_ready.clone();
        let responded = responded_ids.clone();
        let recompile_count = ready_count.clone();
        debug!("spawning stdin->udp thread NOW");
        let handle = thread::Builder::new()
            .name("stdin->udp".into())
            .spawn(move || {
                pump_stdin_to_udp(
                    udp,
                    shutdown,
                    done_tx,
                    ready_flag,
                    responded,
                    recompile_count,
                )
            })
            .context("failed to spawn stdin->udp bridge thread")?;
        debug!("stdin->udp thread spawned successfully");
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
    let max_wait_ms = LSP_READY_MAX_WAIT_MS;
    loop {
        if let Ok(()) = ready_rx.try_recv() {
            let startup_elapsed = startup_start.elapsed();
            debug!(
                "detected 'LSP READY' from sclang (startup: {:.2?})",
                startup_elapsed
            );
            sclang_ready.store(true, Ordering::SeqCst);
            break;
        }
        if waited_ms >= max_wait_ms {
            warn!(
                "timed out waiting for 'LSP READY' ({}s); proceeding anyway",
                max_wait_ms / 1000
            );
            sclang_ready.store(true, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(millis_to_duration(STARTUP_POLL_MS));
        waited_ms += STARTUP_POLL_MS;
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
            .spawn(move || http::run_http_server(port, udp, shutdown))
            .context("failed to spawn HTTP server thread")?
    };

    let status = loop {
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                release_child_state(&child_state);
                break Ok(exit_status);
            }
            Ok(None) => {}
            Err(err) => {
                release_child_state(&child_state);
                break Err(anyhow!("failed to poll sclang status: {err}"));
            }
        }

        if stdin_done_rx.try_recv().is_ok() {
            stdin_closed = true;
            info!("stdin closed, initiating graceful shutdown");

            // First, perform graceful shutdown of sclang (sends LSP shutdown/exit)
            // This gives sclang time to process any final requests before we signal
            // other threads to stop
            let exit_status = graceful_shutdown_child(
                &mut child,
                &udp_sender,
                GRACEFUL_SHUTDOWN_TIMEOUT,
                run_token,
            )
            .context("failed to shut down sclang after stdin closed")?;

            // AFTER sclang has exited, signal threads to stop
            // This ensures the sender thread can deliver final messages while sclang is alive
            shutdown.store(true, Ordering::SeqCst);
            release_child_state(&child_state);
            break Ok(exit_status);
        }

        thread::sleep(millis_to_duration(MAIN_LOOP_POLL_MS));
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

    // Clean up PID file on graceful shutdown
    remove_pid_file();

    if status.success() {
        Ok(())
    } else if stdin_closed {
        debug!("sclang exited after stdin closed ({})", status);
        Ok(())
    } else {
        Err(anyhow!("sclang exited with status {}", status))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_udp_ports_returns_different_ports() {
        let ports = allocate_udp_ports().unwrap();
        assert_ne!(ports.client_port, ports.server_port);
        assert!(ports.client_port > 0);
        assert!(ports.server_port > 0);
    }

    #[test]
    fn test_run_token_increments() {
        let token1 = RUN_TOKEN.fetch_add(1, Ordering::SeqCst);
        let token2 = RUN_TOKEN.fetch_add(1, Ordering::SeqCst);
        assert!(token2 > token1);
    }
}
