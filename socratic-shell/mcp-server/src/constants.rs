//! Constants and configuration values used throughout the Symposium MCP server

/// Default prefix for daemon socket files
pub const DAEMON_SOCKET_PREFIX: &str = "symposium-daemon";

/// Directory for temporary files (sockets, logs, etc.)
pub const TEMP_DIR: &str = "/tmp";

/// Default log file name for development mode
pub const DEV_LOG_FILENAME: &str = "symposium-mcp.log";

/// Default idle timeout for daemon in seconds
pub const DEFAULT_DAEMON_IDLE_TIMEOUT: u64 = 30;

/// Daemon socket path with custom prefix
pub fn daemon_socket_path(prefix: &str) -> String {
    format!("{}/{}.sock", TEMP_DIR, prefix)
}

/// Development log file path
pub fn dev_log_path() -> String {
    format!("{}/{}", TEMP_DIR, DEV_LOG_FILENAME)
}