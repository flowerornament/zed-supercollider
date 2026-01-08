//! Centralized constants for sc_launcher timing, UDP, and LSP configuration.
//!
//! This module contains all timing, network, and protocol constants used throughout
//! the launcher to avoid magic numbers scattered across the codebase.

use std::time::Duration;

// ============================================================================
// UDP & Network Constants
// ============================================================================

/// UDP socket read timeout in milliseconds
pub const UDP_READ_TIMEOUT_MS: u64 = 200;

/// UDP buffer size for receiving messages (64KB)
pub const UDP_BUFFER_SIZE: usize = 64 * 1024;

/// Maximum UDP chunk size to match LanguageServer.quark's maxSize
pub const MAX_UDP_CHUNK_SIZE: usize = 6000;

/// Delay between UDP chunks in microseconds to avoid overwhelming receiver
pub const UDP_CHUNK_DELAY_US: u64 = 100;

// ============================================================================
// Retry & Timeout Constants
// ============================================================================

/// Sleep duration between retry attempts in milliseconds
pub const RETRY_SLEEP_MS: u64 = 50;

/// Maximum total retry duration in milliseconds (90 seconds)
pub const MAX_RETRY_MS: u64 = 90_000;

// ============================================================================
// Polling & Startup Constants
// ============================================================================

/// Main event loop polling interval in milliseconds
pub const MAIN_LOOP_POLL_MS: u64 = 50;

/// Startup polling interval while waiting for LSP READY signal
pub const STARTUP_POLL_MS: u64 = 50;

/// Shutdown polling interval in milliseconds
pub const SHUTDOWN_POLL_MS: u64 = 100;

/// Maximum wait time for LSP READY signal in milliseconds (60 seconds)
pub const LSP_READY_MAX_WAIT_MS: u64 = 60_000;

/// Graceful shutdown timeout duration
pub const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// SIGTERM grace period before forceful termination
pub const SIGTERM_GRACE_PERIOD: Duration = Duration::from_secs(2);

/// Maximum time to wait for sclang ready during shutdown flush (milliseconds)
pub const SHUTDOWN_FLUSH_WAIT_MS: u64 = 2000;

/// Number of retry attempts for LSP shutdown request
pub const SHUTDOWN_RETRY_ATTEMPTS: u32 = 3;

/// Delay between shutdown retry attempts (milliseconds)
pub const SHUTDOWN_RETRY_DELAY_MS: u64 = 100;

// ============================================================================
// LSP & Request ID Constants
// ============================================================================

/// Initial LSP request ID to avoid conflicts with client-generated IDs
pub const INITIAL_LSP_REQUEST_ID: u64 = 1_000_000;

/// Default HTTP server port
pub const DEFAULT_HTTP_PORT: u16 = 57130;

/// JSON-RPC protocol version
pub const JSONRPC_VERSION: &str = "2.0";

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert milliseconds to Duration (const fn for compile-time evaluation)
pub const fn millis_to_duration(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

/// Calculate maximum retry attempts based on timing constants
pub const fn max_retry_attempts() -> usize {
    (MAX_RETRY_MS / RETRY_SLEEP_MS) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_retry_attempts() {
        // 90,000ms / 50ms = 1,800 attempts
        assert_eq!(max_retry_attempts(), 1800);
    }

    #[test]
    fn test_millis_to_duration() {
        assert_eq!(millis_to_duration(100), Duration::from_millis(100));
        assert_eq!(millis_to_duration(0), Duration::from_millis(0));
    }

    #[test]
    fn test_shutdown_timeouts_are_reasonable() {
        // Graceful shutdown should be at least 1 second
        assert!(GRACEFUL_SHUTDOWN_TIMEOUT >= Duration::from_secs(1));

        // SIGTERM grace period should be at least 1 second
        assert!(SIGTERM_GRACE_PERIOD >= Duration::from_secs(1));
    }

    #[test]
    fn test_udp_chunk_size_is_reasonable() {
        // Should be less than typical MTU (1500 bytes) multiplied by a reasonable factor
        // but not so large that it causes fragmentation issues
        assert!(MAX_UDP_CHUNK_SIZE > 1000);
        assert!(MAX_UDP_CHUNK_SIZE < 65000);
    }
}
