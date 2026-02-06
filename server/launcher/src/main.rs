//! SuperCollider Language Server launcher.
//!
//! Bridges Zed's LSP client with sclang's LanguageServer.quark via UDP transport.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use log::debug;
use std::io::Write;
use std::process::Stdio;

use sc_launcher::logging::{debug_file_logs_enabled, log_dir, timestamp};
use sc_launcher::orchestrator::run_lsp_bridge;
use sc_launcher::process::{detect_sclang, make_sclang_command};
use sc_launcher::{Args, Mode};

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<()> {
    // Initialize structured logging
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or("RUST_LOG", "sc_launcher=info"),
    )
    .format_target(false)
    .format_timestamp_millis()
    .init();

    // Write startup log to a file since stderr may be buffered/filtered by Zed
    if debug_file_logs_enabled() {
        let log_path = log_dir().join("sc_launcher_startup.log");
        let startup_log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path);
        if let Ok(mut f) = startup_log {
            let _ = writeln!(f, "\n[{}] ======== MAIN STARTED ========", timestamp());
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

    debug!(
        "PID={} args={:?}",
        std::process::id(),
        std::env::args().collect::<Vec<_>>()
    );

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
            println!("{}", probe);
            Ok(())
        }
        Mode::Lsp => run_lsp_bridge(&sclang, &args),
    }
}
