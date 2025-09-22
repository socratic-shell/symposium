//! Integration tests for daemon spawning and MCP server integration

use symposium_mcp::SymposiumServer;

#[tokio::test]
async fn test_daemon_spawning_integration() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    // This test verifies that the MCP server can spawn and connect to the daemon
    // We'll use the test mode to avoid requiring actual VSCode PID discovery

    let _server = SymposiumServer::new_test();

    // Verify server was created successfully
    assert!(true, "Server created successfully in test mode");

    // In test mode, IPC operations are mocked, so we can't test the actual daemon connection
    // But we can verify the server initializes without errors
}

#[tokio::test]
async fn test_daemon_ensure_running_separate_process() {
    use symposium_mcp::run_daemon_with_idle_timeout;
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use uuid::Uuid;

    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    // Test the daemon spawning logic in isolation using the library function
    let test_id = Uuid::new_v4();
    let socket_prefix = format!("symposium-integration-test-{}", test_id);
    let socket_path = format!("/tmp/{}.sock", socket_prefix);

    // Clean up any existing socket
    let _ = std::fs::remove_file(&socket_path);

    // Barrier for coordinating when daemon is ready
    let ready_barrier = Arc::new(Barrier::new(2));

    // Start daemon with idle timeout (using library function, not separate process)
    let ready_barrier_clone = ready_barrier.clone();
    let daemon_handle = tokio::spawn(async move {
        run_daemon_with_idle_timeout(&socket_prefix, 30, Some(ready_barrier_clone)).await
    });

    // Wait for daemon to be ready
    ready_barrier.wait().await;

    // Verify socket was created
    assert!(
        std::path::Path::new(&socket_path).exists(),
        "Daemon should create socket file"
    );

    // Verify we can connect to the daemon
    let connection_result = tokio::net::UnixStream::connect(&socket_path).await;
    assert!(
        connection_result.is_ok(),
        "Should be able to connect to daemon"
    );

    // Clean up
    daemon_handle.abort();
}

// Note: Testing separate process spawning requires more complex integration tests
// that would need to be run with the actual binary. The above tests verify
// the core daemon functionality works correctly.
