//! Constants and configuration values used throughout the Symposium MCP server

/// Default prefix for daemon socket files
pub const DAEMON_SOCKET_PREFIX: &str = "symposium-daemon";

/// Directory for temporary files (sockets, logs, etc.)
pub const TEMP_DIR: &str = "/tmp";

/// Default log file name for development mode
pub const DEV_LOG_FILENAME: &str = "symposium-mcp.log";

/// Default idle timeout for daemon in seconds
pub const DEFAULT_DAEMON_IDLE_TIMEOUT: u64 = 30;

/// Global daemon socket path (used by all clients and servers)
pub fn global_daemon_socket_path() -> String {
    format!("{}/{}.sock", TEMP_DIR, DAEMON_SOCKET_PREFIX)
}

/// Development log file path
pub fn dev_log_path() -> String {
    format!("{}/{}", TEMP_DIR, DEV_LOG_FILENAME)
}

/// Legacy PID-specific daemon socket path (for backward compatibility during migration)
pub fn pid_specific_daemon_socket_path(pid: u32) -> String {
    format!("{}/{}-{}.sock", TEMP_DIR, DAEMON_SOCKET_PREFIX, pid)
}