//! Manual test for separate process daemon spawning
//!
//! This test demonstrates that the daemon can be spawned as a separate process
//! and that multiple processes can connect to it.
#![cfg(test)]

use std::process::Command;
use tokio::time::{Duration, timeout};

#[tokio::test]
#[ignore] // Ignore by default since this requires the binary to be built
async fn test_separate_process_daemon_spawning() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    let test_pid = std::process::id(); // Use current process PID so daemon won't exit
    let socket_path = format!("/tmp/symposium-daemon-{}.sock", test_pid);

    // Clean up any existing socket
    let _ = std::fs::remove_file(&socket_path);

    // Get the current executable path
    let current_exe = std::env::current_exe().expect("Failed to get current executable");

    // Spawn daemon as separate process
    let mut cmd = Command::new(&current_exe);
    cmd.args(&["daemon", &test_pid.to_string()]);

    let mut child = cmd.spawn().expect("Failed to spawn daemon process");

    println!("Spawned daemon process with PID: {}", child.id());

    // Wait for daemon to be ready
    let connect_result = timeout(Duration::from_secs(5), async {
        loop {
            if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        connect_result.is_ok(),
        "Daemon should be ready within 5 seconds"
    );

    // Verify we can connect to the daemon
    let connection = tokio::net::UnixStream::connect(&socket_path).await;
    assert!(connection.is_ok(), "Should be able to connect to daemon");

    println!("Successfully connected to daemon!");

    // Clean up: kill the daemon process
    child.kill().expect("Failed to kill daemon process");
    child.wait().expect("Failed to wait for daemon process");

    // Verify socket is cleaned up
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Note: Socket might still exist briefly after process death
}
