//! Logging utilities for sc_launcher.
//!
//! Provides timestamp generation, log directory management, and child process
//! stream logging with LSP READY detection.

use log::debug;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

// ============================================================================
// Timestamp Generation
// ============================================================================

/// Generate a timestamp string in format "YYYY-MM-DD HH:MM:SS.mmm".
/// Uses libc for local time conversion to avoid heavy chrono dependency.
pub fn timestamp() -> String {
    use libc::{localtime_r, strftime, time_t, tm};
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get epoch time for local conversion.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let secs = now.as_secs() as time_t;
    let millis = now.subsec_millis();

    // Convert to local time and format YYYY-MM-DD HH:MM:SS.mmm
    // SAFETY: localtime_r is thread-safe (uses caller-provided tm struct)
    let mut tm: tm = unsafe { std::mem::zeroed() };
    unsafe {
        localtime_r(&secs, &mut tm);
    }

    let mut buf = [0u8; 32];
    let fmt = b"%Y-%m-%d %H:%M:%S\0";
    // SAFETY: strftime writes to our buffer, format string is null-terminated
    let len = unsafe {
        strftime(
            buf.as_mut_ptr() as *mut i8,
            buf.len(),
            fmt.as_ptr() as *const i8,
            &tm,
        )
    };
    let prefix = std::str::from_utf8(&buf[..len as usize]).unwrap_or("1970-01-01 00:00:00");
    format!("{prefix}.{millis:03}")
}

// ============================================================================
// Log Directory & Configuration
// ============================================================================

/// Get the log directory path.
/// Prefers SC_TMP_DIR, then TMPDIR, then system temp dir.
pub fn log_dir() -> std::path::PathBuf {
    std::env::var_os("SC_TMP_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Check if debug file logging is enabled (SC_LAUNCHER_DEBUG_LOGS).
pub fn debug_file_logs_enabled() -> bool {
    std::env::var("SC_LAUNCHER_DEBUG_LOGS").is_ok()
}

/// Check if post window logging is enabled (SC_LAUNCHER_POST_LOG != "0").
pub fn post_log_enabled() -> bool {
    std::env::var("SC_LAUNCHER_POST_LOG")
        .map(|v| v != "0")
        .unwrap_or(true)
}

// ============================================================================
// Child Stream Logging
// ============================================================================

/// Log output from a child process stream (stdout/stderr).
///
/// Spawns a thread that reads lines from the stream and:
/// - Logs them to stderr (if verbose or no file logging)
/// - Writes non-LSP lines to sclang_post.log (if post logging enabled)
/// - Signals LSP READY when detected
/// - Increments ready_count for recompile detection
pub fn log_child_stream<R>(
    label: &'static str,
    stream: R,
    ready_signal: Option<mpsc::Sender<()>>,
    ready_count: Option<Arc<AtomicU64>>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
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

            if post_file.is_some() && label == "sclang stdout" {
                debug!("sclang output -> {}", post_log_path.display());
            } else if log_to_file && post_file.is_none() {
                debug!(
                    "warning: failed to open post log at {}",
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
                    // Log child output at debug level (or to stderr if no file logging)
                    if !log_to_file {
                        eprintln!("[{label}] {trimmed}");
                    } else {
                        debug!("[{label}] {trimmed}");
                    }

                    // Write stdout to post window log file (filter out verbose LSP debug messages)
                    if log_to_file {
                        if let Some(ref mut f) = post_file {
                            // Skip LSP protocol noise - only show actual post window content
                            let is_lsp_noise = trimmed.contains("[LANGUAGESERVER.QUARK]")
                                || trimmed.starts_with("{\"") // JSON responses
                                || trimmed.starts_with("Content-Length:");
                            if !is_lsp_noise {
                                let _ = writeln!(f, "{}", trimmed);
                            }
                        }
                    }

                    if label == "sclang stdout" && trimmed.contains("*** LSP READY ***") {
                        if let Some(tx) = &ready_signal {
                            let _ = tx.send(());
                        }
                        // Increment ready count for recompile detection
                        if let Some(ref counter) = ready_count {
                            let old_count = counter.fetch_add(1, Ordering::SeqCst);
                            debug!("LSP READY count: {} -> {}", old_count, old_count + 1);
                        }
                    }
                }
                line.clear();
            }
        })
        .expect("failed to spawn child log thread")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        // Should be in format "YYYY-MM-DD HH:MM:SS.mmm"
        assert!(ts.len() >= 23, "timestamp too short: {}", ts);
        assert!(ts.contains('-'), "timestamp missing date separator: {}", ts);
        assert!(ts.contains(':'), "timestamp missing time separator: {}", ts);
        assert!(ts.contains('.'), "timestamp missing milliseconds: {}", ts);
    }

    #[test]
    fn test_log_dir_returns_path() {
        let dir = log_dir();
        // Should return some path (either from env or system temp)
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_post_log_enabled_default() {
        // Default should be true (when env var not set)
        std::env::remove_var("SC_LAUNCHER_POST_LOG");
        assert!(post_log_enabled());
    }
}
