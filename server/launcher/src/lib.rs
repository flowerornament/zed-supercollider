//! SuperCollider Language Server launcher library.
//!
//! This crate provides the sc_launcher binary that bridges Zed's LSP client
//! with sclang's LanguageServer.quark via UDP transport.
//!
//! # Architecture
//!
//! ```text
//! Zed <-> stdin/stdout <-> sc_launcher <-> UDP <-> sclang (LanguageServer.quark)
//!                              |
//!                              +-> HTTP server (eval requests)
//! ```
//!
//! # Modules
//!
//! - [`bridge`]: LSP protocol bridge between stdin/stdout and UDP
//! - [`constants`]: Timing, network, and protocol constants
//! - [`http`]: HTTP server for eval requests and control commands
//! - [`logging`]: Timestamp generation and child process stream logging
//! - [`orchestrator`]: LSP bridge coordination and sclang lifecycle
//! - [`process`]: Process discovery, PID management, and signal handling

use clap::Parser;

pub mod bridge;
pub mod constants;
pub mod http;
pub mod logging;
pub mod orchestrator;
pub mod process;

// ============================================================================
// CLI Types (shared between main.rs and modules)
// ============================================================================

/// SuperCollider Language Server launcher arguments.
///
/// This struct is shared between main.rs and the library modules that need
/// access to CLI arguments (e.g., process::detect_sclang, orchestrator::run_lsp_bridge).
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
    #[arg(long, default_value_t = constants::DEFAULT_HTTP_PORT)]
    pub http_port: u16,
}

/// Launcher operation mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum Mode {
    /// Probe sclang availability and print JSON
    Probe,
    /// Run the LSP bridge (stdin/stdout ↔ LanguageServer.quark UDP transport)
    Lsp,
}

// Re-exports for public API and backwards compatibility
pub use bridge::{create_execute_command_request, next_lsp_request_id, RequestId};
pub use http::run_http_server;
pub use orchestrator::{graceful_shutdown_child, run_lsp_bridge, IS_RUNNING};
pub use process::{remove_pid_file, write_pid_file};
