use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::process::{Command, Stdio};

/// SuperCollider Language Server launcher
///
/// Responsibilities (stub):
/// - Detect sclang path
/// - Ensure LanguageServer.quark is installed (future)
/// - Launch sclang with LanguageServer and bridge to stdio (future)
#[derive(Parser, Debug)]
#[command(name = "sc_launcher", version, about = "Launch sclang LSP for Zed")] 
struct Args {
    /// Path to sclang executable (overrides detection)
    #[arg(long)]
    sclang_path: Option<String>,

    /// Optional SuperCollider config YAML path
    #[arg(long)]
    conf_yaml_path: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let sclang = match &args.sclang_path {
        Some(p) => p.clone(),
        None => which::which("sclang")
            .map_err(|_| anyhow!("sclang not found on PATH; set --sclang-path"))?
            .display()
            .to_string(),
    };

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
    let json = format!(
        "{{\"ok\":true,\"sclang\":{{\"path\":\"{}\"}},\"note\":\"probe-only; LSP bootstrap TBD\"}}",
        sclang.replace('"', "\\\"")
    );
    println!("{}", json);
    Ok(())
}
