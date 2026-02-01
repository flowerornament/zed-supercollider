//! Process lifecycle management for sc_launcher.
//!
//! Handles sclang process discovery, spawning, PID file management,
//! and cleanup of orphaned processes.

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::logging::{log_dir, verbose_logging_enabled};
use crate::Args;

// ============================================================================
// Safe Signal Wrapper
// ============================================================================

/// Safe wrappers around libc signal operations.
/// All unsafe code is isolated here with SAFETY documentation.
#[cfg(unix)]
pub mod signal {
    use std::io;

    /// Check if a process exists (signal 0 is POSIX standard).
    /// Returns true if the process exists, false otherwise.
    pub fn process_exists(pid: u32) -> bool {
        // SAFETY: kill(pid, 0) only checks existence, no signal sent.
        // This is a standard POSIX operation for checking process existence.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    /// Send SIGTERM for graceful termination.
    /// Returns Ok(()) if signal was sent, Err with OS error otherwise.
    pub fn send_sigterm(pid: u32) -> io::Result<()> {
        // SAFETY: SIGTERM (15) requests graceful termination.
        // Process can catch this signal and clean up.
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Send SIGKILL for immediate termination.
    /// Returns Ok(()) if signal was sent, Err with OS error otherwise.
    pub fn send_sigkill(pid: u32) -> io::Result<()> {
        // SAFETY: SIGKILL (9) terminates process immediately.
        // Process cannot catch or ignore this signal.
        let result = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

// ============================================================================
// PID File Management
// ============================================================================

/// Get the path to the PID file.
pub fn pid_file_path() -> std::path::PathBuf {
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
            eprintln!(
                "[sc_launcher] warning: failed to remove PID file {:?}: {}",
                path, e
            );
        } else if verbose_logging_enabled() {
            eprintln!("[sc_launcher] removed PID file at {:?}", path);
        }
    }
}

// ============================================================================
// Process Lifecycle
// ============================================================================

/// Check if a process is alive.
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        signal::process_exists(pid)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Kill a process by PID.
/// Tries SIGTERM first, then SIGKILL if the process doesn't respond.
pub fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        // Try SIGTERM first
        let _ = signal::send_sigterm(pid);

        // Give it a moment to exit
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if still alive, use SIGKILL if needed
        if is_process_alive(pid) {
            eprintln!(
                "[sc_launcher] sclang {} didn't respond to SIGTERM, using SIGKILL",
                pid
            );
            let _ = signal::send_sigkill(pid);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

// ============================================================================
// Orphan Process Cleanup
// ============================================================================

/// Process IDs from a PID file.
struct PidFileInfo {
    launcher_pid: u64,
    sclang_pid: u64,
}

/// Read and parse the PID file, returning None if file doesn't exist or is malformed.
fn read_pid_file() -> Option<PidFileInfo> {
    let path = pid_file_path();
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let launcher_pid = json.get("launcher_pid")?.as_u64()?;
    let sclang_pid = json.get("sclang_pid")?.as_u64()?;
    Some(PidFileInfo {
        launcher_pid,
        sclang_pid,
    })
}

/// Clean up orphaned sclang processes from previous launcher instances.
/// Called at startup to prevent accumulation of zombie processes.
pub fn cleanup_orphaned_processes() {
    // Check PID file for stale process
    if let Some(info) = read_pid_file() {
        let launcher_alive = is_process_alive(info.launcher_pid as u32);

        if launcher_alive {
            eprintln!(
                "[sc_launcher] warning: another launcher (pid={}) appears to be running",
                info.launcher_pid
            );
        } else {
            // Old launcher is dead - check if sclang is orphaned
            if is_process_alive(info.sclang_pid as u32) {
                if verbose_logging_enabled() {
                    eprintln!(
                        "[sc_launcher] found orphaned sclang (pid={}) from dead launcher (pid={}), killing",
                        info.sclang_pid, info.launcher_pid
                    );
                }
                kill_process(info.sclang_pid as u32);
            }
            // Remove stale PID file
            let _ = std::fs::remove_file(pid_file_path());
        }
    }

    // Also scan for any orphaned sclang/scsynth processes (PPID=1)
    #[cfg(unix)]
    {
        cleanup_orphaned_sclang_by_ppid();
        cleanup_orphaned_scsynth_by_ppid();
    }
}

/// Scan for orphaned processes by name with PPID=1 and kill them.
#[cfg(unix)]
fn cleanup_orphaned_by_ppid(process_name: &str) {
    // Use ps to find processes with PPID=1 (orphaned, reparented to init)
    let output = Command::new("ps").args(["-eo", "pid,ppid,comm"]).output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            // Parse: "  PID  PPID COMM"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(pid), Ok(ppid)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    let comm = parts[2..].join(" ");
                    // Check if it's an orphaned process (PPID=1 means parent died)
                    if ppid == 1 && comm.contains(process_name) {
                        if verbose_logging_enabled() {
                            eprintln!(
                                "[sc_launcher] found orphaned {} process (pid={}, ppid=1), killing",
                                process_name, pid
                            );
                        }
                        kill_process(pid);
                    }
                }
            }
        }
    }
}

/// Scan for orphaned sclang processes (PPID=1) and kill them.
#[cfg(unix)]
fn cleanup_orphaned_sclang_by_ppid() {
    cleanup_orphaned_by_ppid("sclang");
}

/// Scan for orphaned scsynth processes (PPID=1) and kill them.
#[cfg(unix)]
fn cleanup_orphaned_scsynth_by_ppid() {
    cleanup_orphaned_by_ppid("scsynth");
}

// ============================================================================
// sclang Detection & Command Building
// ============================================================================

/// Construct an sclang command, forcing the appropriate architecture slice on macOS.
pub fn make_sclang_command(path: &str) -> Command {
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

/// Detect the sclang executable path.
/// Checks: --sclang-path argument, SCLANG_PATH env, PATH, macOS default location.
pub fn detect_sclang(args: &Args) -> Result<String> {
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
                eprintln!(
                    "[sc_launcher] using default macOS sclang at {}",
                    default_mac
                );
            }
            return Ok(default_mac.to_string());
        }
    }

    Err(anyhow!(
        "sclang not found; set --sclang-path or SCLANG_PATH, or add sclang to PATH"
    ))
}

// ============================================================================
// Quark Path Discovery
// ============================================================================

/// Find the vendored LanguageServer.quark path.
/// Looks relative to the executable and current working directory.
pub fn find_vendored_quark_path() -> Option<String> {
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

/// Get paths to installed LanguageServer quarks.
/// Checks downloaded-quarks and Extensions directories.
pub fn installed_quark_paths() -> Vec<std::path::PathBuf> {
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

/// Check if LanguageServer.quark is present.
pub fn ensure_quark_present() -> bool {
    !installed_quark_paths().is_empty()
}

/// Find the path to the built-in scide_scqt directory containing the ScIDE Document class.
/// This needs to be excluded when using the vendored LanguageServer.quark which provides
/// its own Document class that delegates to LSPDocument.
pub fn find_scide_scqt_path(sclang_path: &str) -> Option<String> {
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_path_is_in_temp_dir() {
        let path = pid_file_path();
        assert!(path.ends_with("sc_launcher.pid"));
    }

    #[test]
    fn test_is_process_alive_returns_bool() {
        // Current process should always be alive
        #[cfg(unix)]
        {
            let alive = is_process_alive(std::process::id());
            assert!(alive, "current process should be alive");
        }

        // Non-existent PID should return false
        let dead = is_process_alive(999999999);
        assert!(!dead, "non-existent process should not be alive");
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_process_exists() {
        // Current process should exist
        let pid = std::process::id();
        assert!(signal::process_exists(pid));

        // Non-existent PID should return false
        assert!(!signal::process_exists(999999999));
    }

    #[test]
    fn test_installed_quark_paths_returns_vec() {
        // Just verify it doesn't panic and returns a vector
        let paths = installed_quark_paths();
        // May be empty if quarks not installed, that's fine
        let _ = paths; // Use the value to verify it's created without panic
    }

    #[test]
    fn test_make_sclang_command_returns_command() {
        let cmd = make_sclang_command("/usr/bin/sclang");
        // Just verify we get a command object
        let _ = cmd;
    }
}
