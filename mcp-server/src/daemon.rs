//! Message bus daemon for multi-window support
//!
//! Provides a Unix domain socket-based message bus that allows multiple
//! MCP servers and VSCode extensions to communicate through a central daemon.

use anyhow::Result;
use std::collections::HashMap;
use tracing::{error, info};
use tokio::time::{Duration, Instant};

/// Spawn the daemon as a separate detached process
pub async fn spawn_daemon_process(vscode_pid: u32) -> Result<()> {
    use std::process::Command;

    let socket_path = format!("/tmp/symposium-daemon-{}.sock", vscode_pid);

    // Check if daemon is already running by trying to connect
    if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
        info!(
            "Message bus daemon already running for VSCode PID {}",
            vscode_pid
        );
        return Ok(());
    }

    info!(
        "Starting message bus daemon as separate process for VSCode PID {}",
        vscode_pid
    );

    // Get the current executable path to spawn daemon
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to get current executable path: {}", e))?;

    // Spawn daemon as separate detached process
    let mut cmd = Command::new(&current_exe);
    cmd.args(&["daemon", &vscode_pid.to_string()]);
    cmd.stdout(std::process::Stdio::piped()); // Capture stdout to read readiness message
    cmd.stderr(std::process::Stdio::null()); // Suppress stderr to avoid noise

    // Detach from parent process (Unix-specific)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // Create new process group
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn daemon process: {}", e))?;

    info!("Spawned daemon process with PID: {}", child.id());

    // Read stdout until we get the "OK" message indicating daemon is ready
    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        use std::time::Duration;

        let reader = BufReader::new(stdout);

        // Use a timeout as a safety net, but rely primarily on the OK message
        let timeout_result = tokio::time::timeout(Duration::from_secs(10), async {
            // We need to use blocking I/O here since we're reading from a process
            tokio::task::spawn_blocking(move || {
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            if line.trim() == "DAEMON_READY" {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Error reading daemon stdout: {}", e));
                        }
                    }
                }
                Err(anyhow::anyhow!(
                    "Daemon process ended without sending DAEMON_READY message"
                ))
            })
            .await?
        })
        .await;

        match timeout_result {
            Ok(Ok(())) => {
                info!("Message bus daemon confirmed ready");
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                anyhow::bail!("Timeout waiting for daemon readiness confirmation (10 seconds)");
            }
        }
    } else {
        anyhow::bail!("Failed to capture daemon stdout for readiness confirmation");
    }
}

/// Run the message bus daemon with custom socket path prefix
/// If ready_barrier is provided, it will be signaled when the daemon is ready to accept connections
pub async fn run_daemon_with_prefix(
    vscode_pid: u32,
    socket_prefix: &str,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> Result<()> {
    use std::os::unix::net::UnixListener;
    use std::path::Path;

    let socket_path = format!("/tmp/{}-{}.sock", socket_prefix, vscode_pid);
    info!("daemon: attempting to claim socket: {}", socket_path);

    // Try to bind to the socket first - this is our "claim" operation
    let _listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => {
            info!("✅ daemon: successfully claimed socket: {}", socket_path);
            listener
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!("❌ daemon: failed to claim socket {}: {}", socket_path, e);
                error!(
                    "Another daemon is already running for VSCode PID {}",
                    vscode_pid
                );
            } else {
                error!("❌ daemon: Failed to claim socket {}: {}", socket_path, e);
            }
            return Err(e.into());
        }
    };

    info!(
        "🚀 daemon: message bus daemon started for VSCode PID {}",
        vscode_pid
    );
    info!("📡 daemon: listening on socket: {}", socket_path);

    // Convert std::os::unix::net::UnixListener to tokio::net::UnixListener
    _listener.set_nonblocking(true)?;
    let listener = tokio::net::UnixListener::from_std(_listener)?;

    // Signal that daemon is ready to accept connections
    println!("DAEMON_READY");

    // Run the message bus loop
    run_message_bus(listener, vscode_pid, ready_barrier).await?;

    // Clean up socket file on exit
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
        info!("🧹 daemon: Cleaned up socket file: {}", socket_path);
    }

    info!("🛑 Daemon shutdown complete");
    Ok(())
}

/// Run the message bus loop - accept connections, broadcast messages, monitor VSCode
pub async fn run_message_bus(
    listener: tokio::net::UnixListener,
    vscode_pid: u32,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> Result<()> {
    use tokio::sync::broadcast;
    use tokio::time::{Duration, interval};

    info!("daemon: starting message bus loop");

    // Signal that daemon is ready to accept connections
    if let Some(barrier) = ready_barrier {
        barrier.wait().await;
    }

    // Broadcast channel for distributing messages to all clients
    let (tx, _rx) = broadcast::channel::<String>(1000);

    // Track connected clients
    let mut clients: HashMap<usize, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut next_client_id = 0;

    // VSCode process monitoring timer
    let mut vscode_check_interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            // Accept new client connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let client_id = next_client_id;
                        next_client_id += 1;

                        info!("daemon: client {} connected", client_id);

                        // Spawn task to handle this client
                        let tx_clone = tx.clone();
                        let rx = tx.subscribe();
                        let handle = tokio::spawn(handle_client(client_id, stream, tx_clone, rx));
                        clients.insert(client_id, handle);
                    }
                    Err(e) => {
                        error!("daemon: failed to accept client connection: {}", e);
                    }
                }
            }

            // Check if VSCode process is still alive
            _ = vscode_check_interval.tick() => {
                match nix::sys::signal::kill(nix::unistd::Pid::from_raw(vscode_pid as i32), None) {
                    Ok(_) => {
                        // Process exists, continue
                    }
                    Err(nix::errno::Errno::ESRCH) => {
                        info!("daemon: VSCode process {} has died, shutting down daemon", vscode_pid);
                        break;
                    }
                    Err(e) => {
                        error!("daemon: Error checking VSCode process {}: {}", vscode_pid, e);
                    }
                }
            }

            // Clean up finished client tasks
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                clients.retain(|&client_id, handle| {
                    if handle.is_finished() {
                        info!("daemon: client {} disconnected", client_id);
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    // Shutdown: wait for all client tasks to finish
    info!(
        "daemon: shutting down message bus, waiting for {} clients",
        clients.len()
    );
    for (client_id, handle) in clients {
        handle.abort();
        info!("daemon: disconnected client {}", client_id);
    }

    Ok(())
}

/// Handle a single client connection - read messages and broadcast them
pub async fn handle_client(
    client_id: usize,
    mut stream: tokio::net::UnixStream,
    tx: tokio::sync::broadcast::Sender<String>,
    mut rx: tokio::sync::broadcast::Receiver<String>,
) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        tokio::select! {
            // Read messages from this client
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        // EOF - client disconnected
                        info!("daemon: client {} disconnected (EOF)", client_id);
                        break;
                    }
                    Ok(_) => {
                        let message = line.trim().to_string();
                        if !message.is_empty() {
                            info!("daemon: client {} sent: {}", client_id, message);

                            // Broadcast message to all other clients
                            if let Err(e) = tx.send(message) {
                                error!("daemon: failed to broadcast message from client {}: {}", client_id, e);
                            }
                        }
                        line.clear();
                    }
                    Err(e) => {
                        error!("daemon: error reading from client {}: {}", client_id, e);
                        break;
                    }
                }
            }

            // Receive broadcasts from other clients
            result = rx.recv() => {
                match result {
                    Ok(message) => {
                        // Send message to this client
                        let message_with_newline = format!("{}\n", message);
                        if let Err(e) = writer.write_all(message_with_newline.as_bytes()).await {
                            error!("Failed to send message to client {}: {}", client_id, e);
                            break;
                        }
                        if let Err(e) = writer.flush().await {
                            error!("Failed to flush message to client {}: {}", client_id, e);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Broadcast channel closed, disconnecting client {}", client_id);
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Client is too slow, skip lagged messages
                        continue;
                    }
                }
            }
        }
    }

    info!("Client {} handler finished", client_id);
}

#[cfg(test)]
mod test {
    use super::run_daemon_with_prefix;

    #[tokio::test]
    async fn test_daemon_message_broadcasting() {
        use std::sync::Arc;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;
        use tokio::sync::Barrier;
        use tokio::time::{Duration, timeout};
        use uuid::Uuid;

        // Initialize tracing for test output
        let _ = tracing_subscriber::fmt::try_init();

        // Use current process PID so daemon won't exit due to "VSCode died"
        let test_pid = std::process::id();
        // Use UUID to ensure unique socket path per test run
        let test_id = Uuid::new_v4();
        let socket_prefix = format!("dialectic-test-{}", test_id);
        let socket_path = format!("/tmp/{}-{}.sock", socket_prefix, test_pid);

        // Clean up any existing socket
        let _ = std::fs::remove_file(&socket_path);

        // Barrier for coordinating when daemon is ready (2 participants: daemon + test)
        let ready_barrier = Arc::new(Barrier::new(2));
        // Barrier for coordinating when both clients are connected and ready (2 participants: clients)
        let client_barrier = Arc::new(Barrier::new(2));

        // Start the full daemon with unique prefix and ready barrier
        let ready_barrier_clone = ready_barrier.clone();
        let daemon_handle = tokio::spawn(async move {
            run_daemon_with_prefix(test_pid, &socket_prefix, Some(ready_barrier_clone)).await
        });

        // Wait for daemon to be ready
        ready_barrier.wait().await;

        // Verify socket was created
        assert!(
            std::path::Path::new(&socket_path).exists(),
            "Daemon should create socket file"
        );

        // Test: Connect two clients and verify message broadcasting
        let socket_path_1 = socket_path.clone();
        let barrier_1 = client_barrier.clone();
        let client1_handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_1)
                .await
                .expect("Client 1 failed to connect");

            // Wait at barrier until both clients are connected
            barrier_1.wait().await;

            // Client 1 sends first, then waits for response
            stream
                .write_all(b"Hello from client 1\n")
                .await
                .expect("Failed to send message");
            stream.flush().await.expect("Failed to flush");

            // Read response from client 2
            let mut reader = BufReader::new(&mut stream);
            let mut response = String::new();

            match timeout(Duration::from_secs(2), reader.read_line(&mut response)).await {
                Ok(Ok(_)) => response.trim().to_string(),
                _ => "NO_RESPONSE".to_string(),
            }
        });

        let socket_path_2 = socket_path.clone();
        let barrier_2 = client_barrier.clone();
        let client2_handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_2)
                .await
                .expect("Client 2 failed to connect");

            // Wait at barrier until both clients are connected
            barrier_2.wait().await;

            // Client 2 waits to receive message from client 1, then responds
            let mut reader = BufReader::new(&mut stream);
            let mut message = String::new();

            let received =
                match timeout(Duration::from_secs(2), reader.read_line(&mut message)).await {
                    Ok(Ok(_)) => message.trim().to_string(),
                    _ => "NO_MESSAGE".to_string(),
                };

            // Send response back to client 1
            stream
                .write_all(b"Hello from client 2\n")
                .await
                .expect("Failed to send response");
            stream.flush().await.expect("Failed to flush");

            received
        });

        // Wait for both clients to complete
        let (client1_response, client2_received) = tokio::join!(client1_handle, client2_handle);

        // Verify message broadcasting worked
        let client1_response = client1_response.expect("Client 1 task failed");
        let client2_received = client2_received.expect("Client 2 task failed");

        // Client 2 should always receive the message from Client 1
        assert_eq!(
            client2_received, "Hello from client 1",
            "Client 2 should receive message from Client 1"
        );

        // Client 1 might receive either its own message (due to broadcast) or Client 2's response
        // Both are valid in a broadcast system - this verifies the broadcast mechanism works
        assert!(
            client1_response == "Hello from client 1" || client1_response == "Hello from client 2",
            "Client 1 should receive either its own message or Client 2's response, got: '{}'",
            client1_response
        );

        // Clean up
        daemon_handle.abort();
    }

    #[tokio::test]
    async fn test_daemon_multiple_clients() {
        use std::sync::Arc;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;
        use tokio::sync::Barrier;
        use tokio::time::{Duration, timeout};
        use uuid::Uuid;

        // Initialize tracing for test output
        let _ = tracing_subscriber::fmt::try_init();

        // Use current process PID
        let test_pid = std::process::id();
        // Use UUID to ensure unique socket path per test run
        let test_id = Uuid::new_v4();
        let socket_prefix = format!("dialectic-test-{}", test_id);
        let socket_path = format!("/tmp/{}-{}.sock", socket_prefix, test_pid);

        // Clean up any existing socket
        let _ = std::fs::remove_file(&socket_path);

        // Barrier for coordinating when daemon is ready (2 participants: daemon + test)
        let ready_barrier = Arc::new(Barrier::new(2));
        // Barrier for coordinating when all clients are connected (1 sender + 2 receivers = 3)
        let client_barrier = Arc::new(Barrier::new(3));

        // Start the full daemon with unique prefix and ready barrier
        let ready_barrier_clone = ready_barrier.clone();
        let daemon_handle = tokio::spawn(async move {
            run_daemon_with_prefix(test_pid, &socket_prefix, Some(ready_barrier_clone)).await
        });

        // Wait for daemon to be ready
        ready_barrier.wait().await;

        // Verify socket was created
        assert!(
            std::path::Path::new(&socket_path).exists(),
            "Daemon should create socket file"
        );

        // Test: One sender, multiple receivers
        let socket_path_sender = socket_path.clone();
        let barrier_sender = client_barrier.clone();
        let sender_handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_sender)
                .await
                .expect("Sender failed to connect");

            // Wait at barrier until all clients are connected
            barrier_sender.wait().await;

            // All clients are now connected and ready, send broadcast message
            stream
                .write_all(b"Broadcast message\n")
                .await
                .expect("Failed to send message");
            stream.flush().await.expect("Failed to flush");
        });

        let socket_path_r1 = socket_path.clone();
        let barrier_r1 = client_barrier.clone();
        let receiver1_handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_r1)
                .await
                .expect("Receiver 1 failed to connect");

            // Wait at barrier until all clients are connected
            barrier_r1.wait().await;

            // Wait for broadcast message from sender
            let mut reader = BufReader::new(&mut stream);
            let mut message = String::new();

            match timeout(Duration::from_secs(2), reader.read_line(&mut message)).await {
                Ok(Ok(_)) => message.trim().to_string(),
                _ => "NO_MESSAGE".to_string(),
            }
        });

        let socket_path_r2 = socket_path.clone();
        let barrier_r2 = client_barrier.clone();
        let receiver2_handle = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&socket_path_r2)
                .await
                .expect("Receiver 2 failed to connect");

            // Wait at barrier until all clients are connected
            barrier_r2.wait().await;

            // Wait for broadcast message from sender
            let mut reader = BufReader::new(&mut stream);
            let mut message = String::new();

            match timeout(Duration::from_secs(2), reader.read_line(&mut message)).await {
                Ok(Ok(_)) => message.trim().to_string(),
                _ => "NO_MESSAGE".to_string(),
            }
        });

        // Wait for all tasks
        let (_, receiver1_msg, receiver2_msg) =
            tokio::join!(sender_handle, receiver1_handle, receiver2_handle);

        // Verify both receivers got the message
        let receiver1_msg = receiver1_msg.expect("Receiver 1 task failed");
        let receiver2_msg = receiver2_msg.expect("Receiver 2 task failed");

        assert_eq!(
            receiver1_msg, "Broadcast message",
            "Receiver 1 should get broadcast"
        );
        assert_eq!(
            receiver2_msg, "Broadcast message",
            "Receiver 2 should get broadcast"
        );

        // Clean up
        daemon_handle.abort();
    }

    #[tokio::test]
    async fn test_daemon_socket_claiming() {
        use std::sync::Arc;
        use tokio::sync::Barrier;
        use uuid::Uuid;

        // Initialize tracing for test output
        let _ = tracing_subscriber::fmt::try_init();

        // Use actual test process PID (so daemon won't exit due to "process died")
        let test_pid = std::process::id();
        // Use UUID to ensure unique socket path per test run
        let test_id = Uuid::new_v4();
        let socket_prefix = format!("dialectic-test-{}", test_id);
        let socket_path = format!("/tmp/{}-{}.sock", socket_prefix, test_pid);

        // Clean up any existing socket
        let _ = std::fs::remove_file(&socket_path);

        // Barrier for coordinating when first daemon is ready (2 participants: daemon + test)
        let ready_barrier = Arc::new(Barrier::new(2));

        // Start first daemon with ready barrier
        let socket_prefix_1 = socket_prefix.clone();
        let ready_barrier_clone = ready_barrier.clone();
        let daemon1_handle = tokio::spawn(async move {
            run_daemon_with_prefix(test_pid, &socket_prefix_1, Some(ready_barrier_clone)).await
        });

        // Wait for first daemon to be ready
        ready_barrier.wait().await;

        // Verify socket was created
        assert!(
            std::path::Path::new(&socket_path).exists(),
            "First daemon should create socket file"
        );

        // Try to start second daemon with same PID and prefix (should fail)
        let socket_prefix_2 = socket_prefix.clone();
        let daemon2_result =
            tokio::spawn(
                async move { run_daemon_with_prefix(test_pid, &socket_prefix_2, None).await },
            )
            .await;

        // Second daemon should fail due to socket conflict
        assert!(daemon2_result.is_ok(), "Task should complete");
        let daemon2_inner_result = daemon2_result.unwrap();
        assert!(
            daemon2_inner_result.is_err(),
            "Second daemon should fail due to socket conflict"
        );

        // Clean up first daemon
        daemon1_handle.abort();
    }
}

/// Run the message bus daemon with idle timeout instead of VSCode PID monitoring
/// Daemon will automatically shut down after idle_timeout seconds of no connected clients
pub async fn run_daemon_with_idle_timeout(
    socket_prefix: &str,
    idle_timeout_secs: u64,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> Result<()> {
    use std::os::unix::net::UnixListener;
    use std::path::Path;

    let socket_path = format!("/tmp/{}.sock", socket_prefix);
    info!("daemon: attempting to claim socket: {}", socket_path);

    // Try to bind to the socket first - this is our "claim" operation
    let _listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => {
            info!("✅ daemon: successfully claimed socket: {}", socket_path);
            listener
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                error!("❌ daemon: failed to claim socket {}: {}", socket_path, e);
                error!("Another daemon is already running");
            } else {
                error!("❌ daemon: Failed to claim socket {}: {}", socket_path, e);
            }
            return Err(e.into());
        }
    };

    info!(
        "🚀 daemon: message bus daemon started with {} second idle timeout",
        idle_timeout_secs
    );
    info!("📡 daemon: listening on socket: {}", socket_path);

    // Convert std::os::unix::net::UnixListener to tokio::net::UnixListener
    _listener.set_nonblocking(true)?;
    let listener = tokio::net::UnixListener::from_std(_listener)?;

    // Signal that daemon is ready to accept connections
    println!("DAEMON_READY");

    // Run the message bus loop with idle timeout
    run_message_bus_with_idle_timeout(listener, idle_timeout_secs, ready_barrier).await?;

    // Clean up socket file on exit
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
        info!("🧹 daemon: Cleaned up socket file: {}", socket_path);
    }

    info!("🛑 Daemon shutdown complete");
    Ok(())
}

/// Run the message bus loop with idle timeout - shuts down when no clients connected for timeout period
async fn run_message_bus_with_idle_timeout(
    listener: tokio::net::UnixListener,
    idle_timeout_secs: u64,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> Result<()> {
    use tokio::sync::broadcast;
    use tokio::time::interval;

    info!("daemon: starting message bus loop with idle timeout");

    // Signal that daemon is ready to accept connections
    if let Some(barrier) = ready_barrier {
        barrier.wait().await;
    }

    // Broadcast channel for distributing messages to all clients
    let (tx, _rx) = broadcast::channel::<String>(1000);

    // Track connected clients
    let mut clients: HashMap<usize, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut next_client_id = 0;

    // Track when we last had connected clients
    let mut last_activity = Instant::now();
    let idle_timeout = Duration::from_secs(idle_timeout_secs);

    // Idle check interval (check every 5 seconds)
    let mut idle_check_interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            // Accept new client connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let client_id = next_client_id;
                        next_client_id += 1;

                        info!("daemon: client {} connected", client_id);
                        
                        // Update activity timestamp
                        last_activity = Instant::now();

                        // Spawn task to handle this client
                        let tx_clone = tx.clone();
                        let rx = tx.subscribe();
                        let handle = tokio::spawn(handle_client(client_id, stream, tx_clone, rx));
                        clients.insert(client_id, handle);
                    }
                    Err(e) => {
                        error!("daemon: failed to accept client connection: {}", e);
                    }
                }
            }

            // Check for idle timeout
            _ = idle_check_interval.tick() => {
                // Clean up finished client tasks first
                clients.retain(|&client_id, handle| {
                    if handle.is_finished() {
                        info!("daemon: client {} disconnected", client_id);
                        false
                    } else {
                        true
                    }
                });

                // If no clients connected and idle timeout exceeded, shutdown
                if clients.is_empty() {
                    let idle_duration = last_activity.elapsed();
                    if idle_duration >= idle_timeout {
                        info!(
                            "daemon: No clients connected for {:.1}s (timeout: {}s), shutting down", 
                            idle_duration.as_secs_f64(), 
                            idle_timeout_secs
                        );
                        break;
                    }
                } else {
                    // We have active clients, update activity timestamp
                    last_activity = Instant::now();
                }
            }
        }
    }

    // Terminate all remaining client connections
    for (client_id, handle) in clients {
        info!("daemon: terminating client {}", client_id);
        handle.abort();
    }

    Ok(())
}
