use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
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
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tiny_http::{Method, Response, Server};

mod constants;
use constants::*;

fn timestamp() -> String {
    use libc::{localtime_r, strftime, time_t, tm};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get epoch time for local conversion.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let secs = now.as_secs() as time_t;
    let millis = now.subsec_millis();

    // Convert to local time and format YYYY-MM-DD HH:MM:SS.mmm
    let mut tm: tm = unsafe { std::mem::zeroed() };
    unsafe {
        localtime_r(&secs, &mut tm);
    }

    let mut buf = [0u8; 32];
    let fmt = b"%Y-%m-%d %H:%M:%S\0";
    let len = unsafe {
        strftime(
            buf.as_mut_ptr() as *mut i8,
            buf.len(),
            fmt.as_ptr() as *const i8,
            &tm,
        )
    };
    let prefix = std::str::from_utf8(&buf[..len as usize]).unwrap_or("1970-01-01 00:00:00");
    format!("{}.{}", prefix, format!("{millis:03}"))
}

/// SuperCollider Language Server launcher
///
/// Responsibilities:
/// - Detect sclang path.
/// - Warn when LanguageServer.quark is absent.
/// - Launch sclang with LanguageServer enabled and bridge UDP↔stdio.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Mode {
    /// Probe sclang availability and print JSON
    Probe,
    /// Run the LSP bridge (stdin/stdout ↔ LanguageServer.quark UDP transport)
    Lsp,
}

/// Type-safe request ID representation supporting both number and string IDs
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    /// Extract a RequestId from a JSON value
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

#[derive(Parser, Debug)]
#[command(name = "sc_launcher", version, about = "Launch sclang LSP for Zed")]
pub struct Args {
    /// Path to sclang executable (overrides detection)
    #[arg(long)]
    pub sclang_path: Option<String>,

    /// Optional SuperCollider config YAML path
    #[arg(long)]
    pub conf_yaml_path: Option<String>,

    /// Launcher mode
    #[arg(long, value_enum, default_value_t = Mode::Probe)]
    pub mode: Mode,

    /// Optional LSP log level forwarded to LanguageServer.quark (e.g. error, warn, info, debug)
    #[arg(long, value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// HTTP server port for eval requests (0 = auto-assign, default 57130)
    #[arg(long, default_value_t = DEFAULT_HTTP_PORT)]
    pub http_port: u16,
}

fn log_dir() -> std::path::PathBuf {
    std::env::var_os("SC_TMP_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
}

fn debug_file_logs_enabled() -> bool {
    std::env::var("SC_LAUNCHER_DEBUG_LOGS").is_ok()
}

fn verbose_logging_enabled() -> bool {
    debug_file_logs_enabled() || std::env::var("SC_LAUNCHER_DEBUG").is_ok()
}

fn post_log_enabled() -> bool {
    std::env::var("SC_LAUNCHER_POST_LOG")
        .map(|v| v != "0")
        .unwrap_or(true)
}

fn pid_file_path() -> std::path::PathBuf {
    log_dir().join("sc_launcher.pid")
}

/// Write PID file with launcher and sclang PIDs for safe cleanup.
/// Returns Ok(()) on success, Err on failure (non-fatal, just logged).
pub fn write_pid_file(launcher_pid: u32, sclang_pid: u32) -> Result<()> {
    let path = pid_file_path();
    let content = serde_json::json!({
        "launcher_pid": launcher_pid,
        "sclang_pid": sclang_pid
    });
    std::fs::write(&path, content.to_string())
        .with_context(|| format!("failed to write PID file at {:?}", path))?;
    if verbose_logging_enabled() {
        eprintln!("[sc_launcher] wrote PID file at {:?}", path);
    }
    Ok(())
}

/// Remove PID file on graceful shutdown.
pub fn remove_pid_file() {
    let path = pid_file_path();
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("[sc_launcher] warning: failed to remove PID file {:?}: {}", path, e);
        } else if verbose_logging_enabled() {
            eprintln!("[sc_launcher] removed PID file at {:?}", path);
        }
    }
}

/// Clean up orphaned sclang processes from previous launcher instances.
/// Called at startup to prevent accumulation of zombie processes.
pub fn cleanup_orphaned_processes() {
    let path = pid_file_path();

    // Check PID file for stale process
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let launcher_pid = json.get("launcher_pid").and_then(|v| v.as_u64());
                let sclang_pid = json.get("sclang_pid").and_then(|v| v.as_u64());

                if let (Some(launcher_pid), Some(sclang_pid)) = (launcher_pid, sclang_pid) {
                    // Check if the old launcher is still running
                    let launcher_alive = is_process_alive(launcher_pid as u32);

                    if !launcher_alive {
                        // Old launcher is dead - check if sclang is orphaned
                        if is_process_alive(sclang_pid as u32) {
                            if verbose_logging_enabled() {
                                eprintln!(
                                    "[sc_launcher] found orphaned sclang (pid={}) from dead launcher (pid={}), killing",
                                    sclang_pid, launcher_pid
                                );
                            }
                            kill_process(sclang_pid as u32);
                        }
                        // Remove stale PID file
                        let _ = std::fs::remove_file(&path);
                    } else {
                        eprintln!(
                            "[sc_launcher] warning: another launcher (pid={}) appears to be running",
                            launcher_pid
                        );
                    }
                }
            }
        }
    }

    // Also scan for any orphaned sclang processes (PPID=1) with our command signature
    #[cfg(unix)]
    cleanup_orphaned_sclang_by_ppid();
}

/// Check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks if process exists without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Kill a process by PID
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        unsafe {
            // Try SIGTERM first
            libc::kill(pid as i32, libc::SIGTERM);
        }
        // Give it a moment to exit
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if still alive, use SIGKILL if needed
        if is_process_alive(pid) {
            eprintln!("[sc_launcher] sclang {} didn't respond to SIGTERM, using SIGKILL", pid);
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}

/// Scan for orphaned sclang processes (PPID=1) and kill them
#[cfg(unix)]
fn cleanup_orphaned_sclang_by_ppid() {
    use std::process::Command;

    // Use ps to find sclang processes with PPID=1 (orphaned, reparented to init)
    let output = Command::new("ps")
        .args(["-eo", "pid,ppid,comm"])
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            // Parse: "  PID  PPID COMM"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(pid), Ok(ppid)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let comm = parts[2..].join(" ");
                    // Check if it's an orphaned sclang (PPID=1 means parent died)
                    if ppid == 1 && comm.contains("sclang") {
                        if verbose_logging_enabled() {
                            eprintln!(
                                "[sc_launcher] found orphaned sclang process (pid={}, ppid=1), killing",
                                pid
                            );
                        }
                        kill_process(pid);
                    }
                }
            }
        }
    }
}

/// Construct an sclang command, forcing the appropriate architecture slice on macOS.
fn make_sclang_command(path: &str) -> Command {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "x86_64") {
            let mut cmd = Command::new("arch");
            cmd.arg("-x86_64").arg(path);
            return cmd;
        }
    }

    Command::new(path)
}

fn detect_sclang(args: &Args) -> Result<String> {
    if let Some(path) = &args.sclang_path {
        return Ok(path.clone());
    }

    if let Ok(env_path) = std::env::var("SCLANG_PATH") {
        if Path::new(&env_path).exists() {
            if verbose_logging_enabled() {
                eprintln!("[sc_launcher] using sclang from SCLANG_PATH={}", env_path);
            }
            return Ok(env_path);
        }
    }

    if let Ok(path) = which::which("sclang") {
        return Ok(path.display().to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let default_mac = "/Applications/SuperCollider.app/Contents/MacOS/sclang";
        if Path::new(default_mac).exists() {
            if verbose_logging_enabled() {
                eprintln!("[sc_launcher] using default macOS sclang at {}", default_mac);
            }
            return Ok(default_mac.to_string());
        }
    }

    Err(anyhow!(
        "sclang not found; set --sclang-path or SCLANG_PATH, or add sclang to PATH"
    ))
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
                "\n[{}] ======== MAIN STARTED ========",
                timestamp()
            );
            let _ = writeln!(
                f,
                "[{}] PID={} args={:?}",
                timestamp(),
                std::process::id(),
                std::env::args().collect::<Vec<_>>()
            );
            let _ = writeln!(f, "[{}] exe={:?}", timestamp(), std::env::current_exe());
            let _ = writeln!(f, "[{}] log_dir={:?}", timestamp(), log_dir());
        }
    }

    // Log startup details only in verbose mode
    if verbose_logging_enabled() {
        eprintln!("[sc_launcher] ======== MAIN STARTED ========");
        eprintln!(
            "[sc_launcher] PID={} args={:?}",
            std::process::id(),
            std::env::args().collect::<Vec<_>>()
        );
        let _ = std::io::stderr().flush();
    }

    let args = Args::parse();

    let sclang = detect_sclang(&args)?;

    match args.mode {
        Mode::Probe => {
            // For now, just run `sclang -v` to confirm availability.
            let output = make_sclang_command(&sclang)
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

pub fn run_lsp_bridge(sclang: &str, args: &Args) -> Result<()> {
    let startup_start = Instant::now();
    let verbose = verbose_logging_enabled();

    // Clean up any orphaned sclang processes from previous launcher instances
    cleanup_orphaned_processes();

    let run_token = RUN_TOKEN.fetch_add(1, Ordering::SeqCst);
    if IS_RUNNING.swap(true, Ordering::SeqCst) {
        eprintln!(
            "[sc_launcher] run token {}: launcher already running; refusing second spawn",
            run_token
        );
        return Err(anyhow!(
            "sc_launcher already running (token {}) - refusing duplicate spawn",
            run_token
        ));
    }
    let _run_guard = RunningGuard { run_token };
    // Log version at startup to confirm which binary is running
    eprintln!(
        "[sc_launcher] v{} starting LSP bridge (pid={}, run={})",
        env!("CARGO_PKG_VERSION"),
        std::process::id(),
        run_token
    );

    let quark_ok = ensure_quark_present();
    if !quark_ok {
        eprintln!("[sc_launcher] warning: LanguageServer.quark not found in downloaded-quarks; install it via SuperCollider's Quarks GUI or `Quarks.install(\"LanguageServer\");`");
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
        if verbose {
            eprintln!("[sc_launcher] including vendored LanguageServer.quark at {}", vendor_path);
        }
        command.arg("--include-path").arg(&vendor_path);

        for installed in installed_quark_paths() {
            if verbose {
                eprintln!(
                    "[sc_launcher] excluding installed LanguageServer.quark at {}",
                    installed.display()
                );
            }
            command
                .arg("--exclude-path")
                .arg(installed.display().to_string());
        }

        // Exclude the built-in ScIDE Document class so the vendored LSPDocument takes precedence.
        // The vendored quark provides its own Document class in scide_vscode/ that properly
        // delegates to LSPDocument for LSP-based document management.
        if let Some(scide_path) = find_scide_scqt_path(sclang) {
            if verbose {
                eprintln!(
                    "[sc_launcher] excluding built-in scide_scqt at {}",
                    scide_path
                );
            }
            command.arg("--exclude-path").arg(scide_path);
        }
    }

    if verbose {
        eprintln!(
            "[sc_launcher] spawning sclang (client={}, server={}, log_level={})",
            ports.client_port,
            ports.server_port,
            args.log_level
                .as_deref()
                .unwrap_or("error (LanguageServer default)")
        );
    }

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
        if verbose {
            eprintln!(
                "[sc_launcher] run token {}: spawned sclang pid={}",
                run_token, pid
            );
        }
        // Write PID file for safe cleanup by external tools
        if let Err(e) = write_pid_file(std::process::id(), pid) {
            eprintln!("[sc_launcher] warning: {}", e);
        }
    }

    // Wait for LSP READY signal from sclang stdout before pumping stdin to UDP
    let (ready_tx, ready_rx) = mpsc::channel();
    // Track ready count for recompile detection (increments each time LSP READY is seen)
    let ready_count: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let stdout_handle = child
        .stdout
        .take()
        .map(|stream| log_child_stream("sclang stdout", stream, Some(ready_tx.clone()), Some(ready_count.clone())));
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
    if verbose {
        eprintln!("[sc_launcher] about to spawn stdin_bridge thread");
        let _ = std::io::stderr().flush();
    }
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
        if verbose {
            eprintln!("[sc_launcher] spawning stdin->udp thread NOW");
            let _ = std::io::stderr().flush();
        }
        let handle = thread::Builder::new()
            .name("stdin->udp".into())
            .spawn(move || pump_stdin_to_udp(udp, shutdown, done_tx, ready_flag, responded, recompile_count))
            .context("failed to spawn stdin->udp bridge thread")?;
        if verbose {
            eprintln!("[sc_launcher] stdin->udp thread spawned successfully");
            let _ = std::io::stderr().flush();
        }
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
            if verbose {
                eprintln!(
                    "[sc_launcher] detected 'LSP READY' from sclang (startup: {:.2?})",
                    startup_elapsed
                );
            }
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
            .spawn(move || run_http_server(port, udp, shutdown))
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
            eprintln!("[sc_launcher] stdin closed, initiating graceful shutdown");

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
        if verbose {
            eprintln!(
                "[sc_launcher] sclang exited after stdin closed ({})",
                status
            );
        }
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
            return Some(candidate.display().to_string());
        }
    }

    if verbose_logging_enabled() {
        eprintln!("[sc_launcher] no vendored LanguageServer.quark found in expected locations");
    }
    None
}

fn log_child_stream<R>(
    label: &'static str,
    stream: R,
    ready_signal: Option<mpsc::Sender<()>>,
    ready_count: Option<Arc<AtomicU64>>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    let verbose = verbose_logging_enabled();
    let log_to_file = post_log_enabled();
    thread::Builder::new()
        .name(format!("{label}-reader"))
        .spawn(move || {
            // Open post window log file for user-visible output
            let post_log_path = log_dir().join("sclang_post.log");
            let mut post_file = if log_to_file {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&post_log_path)
                    .ok()
            } else {
                None
            };

            if post_file.is_some() && label == "sclang stdout" && verbose {
                eprintln!(
                    "[sc_launcher] sclang output -> {}",
                    post_log_path.display()
                );
            } else if log_to_file && post_file.is_none() {
                eprintln!(
                    "[sc_launcher] warning: failed to open post log at {}",
                    post_log_path.display()
                );
            }

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                if !trimmed.is_empty() {
                    if verbose || !log_to_file {
                        eprintln!("[{label}] {trimmed}");
                    }

                    // Write stdout to post window log file (filter out verbose LSP debug messages)
                    if log_to_file {
                        if let Some(ref mut f) = post_file {
                            // Skip LSP protocol noise - only show actual post window content
                            let is_lsp_noise = trimmed.contains("[LANGUAGESERVER.QUARK]")
                                || trimmed.starts_with("{\"")  // JSON responses
                                || trimmed.starts_with("Content-Length:");
                            if !is_lsp_noise {
                                let _ = writeln!(f, "{}", trimmed);
                            }
                        }
                    }

                    if label == "sclang stdout" && trimmed.contains("***LSP READY***") {
                        if let Some(tx) = &ready_signal {
                            let _ = tx.send(());
                        }
                        // Increment ready count for recompile detection
                        if let Some(ref counter) = ready_count {
                            let old_count = counter.fetch_add(1, Ordering::SeqCst);
                            if verbose {
                                eprintln!("[sc_launcher] LSP READY count: {} -> {}", old_count, old_count + 1);
                            }
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

struct ChildState {
    pid: u32,
    run_token: u64,
    owned: AtomicBool,
}

struct RunningGuard {
    run_token: u64,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        IS_RUNNING.store(false, Ordering::SeqCst);
        if verbose_logging_enabled() {
            eprintln!(
                "[sc_launcher] run token {}: cleared running guard",
                self.run_token
            );
        }
    }
}

/// Monotonic token to tag each launcher run for log disambiguation.
static RUN_TOKEN: AtomicU64 = AtomicU64::new(1);
/// Simple guard to prevent multiple concurrent sclang spawns from this process.
pub static IS_RUNNING: AtomicBool = AtomicBool::new(false);

fn release_child_state(state: &Arc<Mutex<Option<ChildState>>>) {
    if let Ok(mut slot) = state.lock() {
        if let Some(child) = slot.take() {
            child.owned.store(false, Ordering::SeqCst);
            if verbose_logging_enabled() {
                eprintln!(
                    "[sc_launcher] run token {}: released tracked sclang pid {}",
                    child.run_token, child.pid
                );
            }
        }
    }
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
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::INCREMENTAL),
            will_save: None,
            will_save_wait_until: None,
            save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                include_text: None,
            })),
        })),
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

/// Create a typed LSP request with automatic JSON-RPC envelope
fn create_lsp_request<P: serde::Serialize>(
    id: u64,
    method: &str,
    params: P,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "method": method,
        "params": params
    })
}

/// Create a typed LSP notification (no id field)
fn create_lsp_notification<P: serde::Serialize>(
    method: &str,
    params: P,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": JSONRPC_VERSION,
        "method": method,
        "params": params
    })
}

/// Create a workspace/executeCommand request with type-safe arguments
fn create_execute_command_request(
    id: u64,
    command: &str,
    arguments: Vec<serde_json::Value>,
) -> serde_json::Value {
    let params = lsp_types::ExecuteCommandParams {
        command: command.to_string(),
        arguments,
        work_done_progress_params: Default::default(),
    };
    create_lsp_request(id, "workspace/executeCommand", params)
}

pub fn graceful_shutdown_child(
    child: &mut std::process::Child,
    udp_socket: &UdpSocket,
    timeout: Duration,
    run_token: u64,
) -> Result<std::process::ExitStatus> {
    let pid = child.id();
    eprintln!(
        "[sc_launcher] run token {}: initiating graceful shutdown for pid {} (timeout {:?})",
        run_token, pid, timeout
    );

    // Attempt LSP shutdown with retries
    let mut shutdown_sent = false;
    for attempt in 1..=SHUTDOWN_RETRY_ATTEMPTS {
        match request_lsp_shutdown_with_result(udp_socket) {
            Ok(_) => {
                shutdown_sent = true;
                eprintln!(
                    "[sc_launcher] LSP shutdown/exit sent successfully (attempt {})",
                    attempt
                );
                break;
            }
            Err(e) => {
                eprintln!(
                    "[sc_launcher] LSP shutdown attempt {} failed: {}",
                    attempt, e
                );
                if attempt < SHUTDOWN_RETRY_ATTEMPTS {
                    thread::sleep(millis_to_duration(SHUTDOWN_RETRY_DELAY_MS));
                }
            }
        }
    }

    if !shutdown_sent {
        eprintln!("[sc_launcher] WARNING: could not send LSP shutdown, will rely on SIGTERM");
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
                eprintln!("[sc_launcher] sclang exited gracefully with {}", status);
                return Ok(status);
            }
            Ok(None) => {}
            Err(err) => return Err(anyhow!("failed to poll sclang status: {err}")),
        }
        thread::sleep(millis_to_duration(SHUTDOWN_POLL_MS));
    }

    #[cfg(unix)]
    {
        let pid = child.id();
        eprintln!(
            "[sc_launcher] run token {}: sending SIGTERM to sclang pid {}",
            run_token, pid
        );
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }

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

    eprintln!(
        "[sc_launcher] run token {}: forcing sclang shutdown with kill",
        run_token
    );
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
    responded_ids: Arc<Mutex<HashSet<RequestId>>>,
    ready_count: Arc<AtomicU64>,
) -> Result<()> {
    let verbose = verbose_logging_enabled();
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
        use std::io::Write;
        let _ = writeln!(
            f,
            "\n[{}] === pump_stdin_to_udp ENTERED ===",
            timestamp()
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
    let resend_initialize = cached_initialize.clone();
    let recompile_counter = ready_count.clone();
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
            let mut last_ready_count: u64 = 0;

            loop {
                // Check for recompile (ready count increased beyond initial)
                let current_ready_count = recompile_counter.load(Ordering::SeqCst);
                if current_ready_count > last_ready_count {
                    if last_ready_count > 0 {
                        // This is a recompile (not the initial ready)
                        eprintln!("[sc_launcher] RECOMPILE DETECTED (ready count {} -> {}), re-sending initialize",
                            last_ready_count, current_ready_count);
                        // Re-send cached initialize
                        if let Some(init_msg) = resend_initialize.lock().ok().and_then(|m| m.clone()) {
                            eprintln!("[sc_launcher] re-sending cached initialize after recompile");
                            if let Err(err) = send_with_retry(&sender_socket, &init_msg) {
                                eprintln!("[sc_launcher] failed to re-send initialize: {err}");
                            }
                        }
                        // Re-send cached didOpen
                        if let Some(open_msg) = resend_did_open.lock().ok().and_then(|m| m.clone()) {
                            eprintln!("[sc_launcher] re-sending cached didOpen after recompile");
                            if let Err(err) = send_with_retry(&sender_socket, &open_msg) {
                                eprintln!("[sc_launcher] failed to re-send didOpen: {err}");
                            }
                        }
                        // Re-send cached didChange
                        if let Some(change_msg) = resend_did_change.lock().ok().and_then(|m| m.clone()) {
                            eprintln!("[sc_launcher] re-sending cached didChange after recompile");
                            if let Err(err) = send_with_retry(&sender_socket, &change_msg) {
                                eprintln!("[sc_launcher] failed to re-send didChange: {err}");
                            }
                        }
                    }
                    last_ready_count = current_ready_count;
                }

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
                match msg_rx.recv_timeout(millis_to_duration(STARTUP_POLL_MS)) {
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
                        // Reader thread closed - handle shutdown gracefully
                        eprintln!(
                            "[sc_launcher] sender thread: channel disconnected, {} pending messages",
                            pending_messages.len()
                        );

                        if !pending_messages.is_empty() {
                            if ready_signaled {
                                // sclang is ready, flush all pending messages
                                eprintln!(
                                    "[sc_launcher] flushing {} pending messages before shutdown",
                                    pending_messages.len()
                                );
                                for msg in pending_messages.drain(..) {
                                    let _ = send_with_retry(&sender_socket, &msg);
                                }
                            } else {
                                // sclang not ready - wait briefly for ready signal, then decide
                                let deadline = std::time::Instant::now()
                                    + millis_to_duration(SHUTDOWN_FLUSH_WAIT_MS);
                                while std::time::Instant::now() < deadline {
                                    if sender_ready.load(Ordering::SeqCst) {
                                        eprintln!(
                                            "[sc_launcher] sclang became ready during shutdown, flushing {} messages",
                                            pending_messages.len()
                                        );
                                        for msg in pending_messages.drain(..) {
                                            let _ = send_with_retry(&sender_socket, &msg);
                                        }
                                        break;
                                    }
                                    std::thread::sleep(millis_to_duration(STARTUP_POLL_MS));
                                }
                                if !pending_messages.is_empty() {
                                    eprintln!(
                                        "[sc_launcher] WARNING: dropping {} messages - sclang never became ready",
                                        pending_messages.len()
                                    );
                                }
                            }
                        }
                        break;
                    }
                }

                if sender_shutdown.load(Ordering::SeqCst) {
                    // Drain any remaining messages from channel before exiting
                    while let Ok(message) = msg_rx.try_recv() {
                        if ready_signaled {
                            let _ = send_with_retry(&sender_socket, &message);
                        } else {
                            pending_messages.push(message);
                        }
                    }
                    // Final flush attempt if ready
                    if ready_signaled && !pending_messages.is_empty() {
                        eprintln!(
                            "[sc_launcher] sender thread: flushing {} remaining messages on shutdown",
                            pending_messages.len()
                        );
                        for msg in pending_messages.drain(..) {
                            let _ = send_with_retry(&sender_socket, &msg);
                        }
                    } else if !pending_messages.is_empty() {
                        eprintln!(
                            "[sc_launcher] sender thread: dropping {} messages on shutdown (sclang not ready)",
                            pending_messages.len()
                        );
                    }
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
            "[{}] stdin reader: starting main loop",
            timestamp()
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
                        "[{}] MSG#{} ({} bytes): {}",
                        timestamp(),
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
                                                "[{}] >>> RESPONDED TO INITIALIZE: {}",
                                                timestamp(),
                                                response_json
                                            );
                                        }

                                        // Record that we've already responded to this request ID
                                        // so we can suppress sclang's duplicate response
                                        if let Some(request_id) = RequestId::from_json(id) {
                                            if let Ok(mut set) = responded_ids.lock() {
                                                set.insert(request_id.clone());
                                                if verbose {
                                                    eprintln!("[sc_launcher] recorded responded id={} for suppression", request_id);
                                                }
                                            }
                                        }

                                        // Still forward to sclang so it can set up its state
                                        // but we've already responded to Zed

                                        // Cache initialize for re-sending after recompile
                                        if let Ok(mut slot) = cached_initialize.lock() {
                                            *slot = Some(message.clone());
                                        }
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
    responded_ids: Arc<Mutex<HashSet<RequestId>>>,
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
                                if let Some(request_id) = RequestId::from_json(id) {
                                    if let Ok(set) = responded_ids.lock() {
                                        if set.contains(&request_id) {
                                            should_suppress = true;
                                            if verbose {
                                                eprintln!(
                                                    "[sc_launcher] SUPPRESSING duplicate response for id={} (already responded from launcher)",
                                                    request_id
                                                );
                                            }
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
    let verbose = verbose_logging_enabled();

    let mut attempts = 0usize;
    let max_attempts = max_retry_attempts();

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
    if message.len() <= MAX_UDP_CHUNK_SIZE {
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
                    std::thread::sleep(millis_to_duration(RETRY_SLEEP_MS));
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
            (message.len() + MAX_UDP_CHUNK_SIZE - 1) / MAX_UDP_CHUNK_SIZE
        );
    }

    let mut offset = 0;
    while offset < message.len() {
        let end = (offset + MAX_UDP_CHUNK_SIZE).min(message.len());
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
static NEXT_LSP_REQUEST_ID: AtomicU64 = AtomicU64::new(INITIAL_LSP_REQUEST_ID);

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
pub fn run_http_server(
    port: u16,
    udp_socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
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
        let lsp_request = create_execute_command_request(
            request_id,
            "supercollider.eval",
            vec![serde_json::json!(body)],
        );

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
    let lsp_request = create_execute_command_request(request_id, command, args);

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

/// Send LSP shutdown and exit requests, returning Result for retry handling
fn request_lsp_shutdown_with_result(udp_socket: &UdpSocket) -> io::Result<()> {
    let shutdown_id = next_lsp_request_id();
    let shutdown_request = create_lsp_request(shutdown_id, "shutdown", serde_json::json!({}));
    send_lsp_payload(udp_socket, &shutdown_request)?;

    let exit_notification = create_lsp_notification("exit", serde_json::json!({}));
    send_lsp_payload(udp_socket, &exit_notification)?;

    Ok(())
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
