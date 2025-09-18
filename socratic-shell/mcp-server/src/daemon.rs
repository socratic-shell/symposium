//! Message bus daemon for multi-window support
//!
//! Provides a Unix domain socket-based message bus that allows multiple
//! MCP servers and VSCode extensions to communicate through a central daemon.

use anyhow::Result;
use std::collections::HashMap;
use std::pin::pin;
use tokio::signal;
use tokio::time::{Duration, Instant};
use tracing::{error, info};

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
                        info!("client {} disconnected (EOF)", client_id);
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

/// Run the message bus daemon with idle timeout instead of VSCode PID monitoring
/// Daemon will automatically shut down after idle_timeout seconds of no connected clients
pub async fn run_daemon_with_idle_timeout(
    socket_prefix: &str,
    idle_timeout_secs: u64,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
) -> Result<()> {
    use std::os::unix::net::UnixListener;
    use std::path::Path;

    let socket_path = crate::constants::daemon_socket_path(socket_prefix);
    info!("daemon: attempting to claim socket: {}", socket_path);

    // Try to bind to the socket first - this is our "claim" operation
    let _listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => {
            info!("✅ daemon: successfully claimed socket: {}", socket_path);

            // Clear debug logs on successful bind (indicates fresh debug session)
            let log_path = crate::constants::dev_log_path();
            if std::path::Path::new(&log_path).exists() {
                if let Err(e) = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&log_path)
                {
                    info!("⚠️  Could not clear previous debug log: {}", e);
                } else {
                    info!("🧹 Cleared previous debug log for fresh session");
                }
            }

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

    // Set up graceful shutdown handling
    let socket_path_for_cleanup = socket_path.clone();

    // Create signal handlers
    let ctrl_c = signal::ctrl_c();

    let mut sigterm = {
        #[cfg(unix)]
        {
            signal::unix::signal(signal::unix::SignalKind::terminate())?
        }

        #[cfg(not(unix))]
        {
            compile_error!("TODO: non-unix support")
        }
    };

    let shutdown = async move {
        tokio::select! {
            // Handle SIGTERM/SIGINT for graceful shutdown
            _ = ctrl_c => {
                info!("🛑 Received SIGINT (Ctrl+C), shutting down gracefully...");
            }
            _ = sigterm.recv() => {
                info!("🛑 Received SIGTERM, shutting down gracefully...");
            }
        }
    };

    let shutdown_result =
        run_message_bus_with_shutdown_signal(listener, idle_timeout_secs, ready_barrier, shutdown)
            .await;

    // Clean up socket file on exit
    if Path::new(&socket_path_for_cleanup).exists() {
        std::fs::remove_file(&socket_path_for_cleanup)?;
        info!("🧹 Cleaned up socket file: {}", socket_path_for_cleanup);
    }

    info!("🛑 Daemon shutdown complete");

    // Return the shutdown result (could be an error from the message bus loop)
    shutdown_result
}

/// Run the message bus loop with idle timeout and shutdown signal
/// Shuts down when no clients connected for timeout period OR when shutdown signal received
async fn run_message_bus_with_shutdown_signal(
    listener: tokio::net::UnixListener,
    idle_timeout_secs: u64,
    ready_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
    shutdown: impl Future<Output = ()>,
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

    let mut shutdown = pin!(async move { shutdown.await });

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

            // Handle shutdown signal (SIGTERM/SIGINT)
            () = &mut shutdown => {
                info!("🔄 Daemon received shutdown signal, broadcasting reload_window to all clients");

                // Create reload_window message
                use crate::types::{IPCMessage, IPCMessageType};
                use serde_json::json;
                use uuid::Uuid;

                let reload_message = IPCMessage {
                    message_type: IPCMessageType::ReloadWindow,
                    id: Uuid::new_v4().to_string(),
                    sender: crate::types::MessageSender {
                        working_directory: "/tmp".to_string(), // Broadcast message
                        taskspace_uuid: None,
                        shell_pid: None,
                    },
                    payload: json!({}), // Empty payload
                };

                // Broadcast reload message to all connected clients
                if let Ok(message_json) = serde_json::to_string(&reload_message) {
                    if let Err(e) = tx.send(message_json) {
                        info!("No clients to receive reload signal: {}", e);
                    } else {
                        info!("✅ Broadcast reload_window message to all clients");
                        // Give clients a moment to process the reload message
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                } else {
                    error!("Failed to serialize reload_window message");
                }

                break; // Exit the message bus loop
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

/// Run as client - connects to daemon and bridges stdin/stdout using actors
/// If auto_start is true and daemon is not running, spawns an independent daemon process
pub async fn run_client(socket_prefix: &str, auto_start: bool) -> Result<()> {
    use crate::actor::{spawn_client, StdioHandle};

    info!("🔌 Starting client with actor-based architecture");

    // Create ClientActor - returns channels directly
    let (to_daemon_tx, mut from_daemon_rx) = spawn_client(
        socket_prefix.to_string(),
        auto_start,
    );

    // Create StdioActor - needs sender to send TO daemon, returns sender for messages FROM daemon
    let (_stdio_handle, to_stdout_tx) = StdioHandle::new(to_daemon_tx);

    // Wire messages from daemon to stdio for stdout
    tokio::spawn(async move {
        while let Some(message) = from_daemon_rx.recv().await {
            if let Err(e) = to_stdout_tx.send(message).await {
                tracing::error!("Failed to forward daemon message to stdout: {}", e);
                break;
            }
        }
    });

    info!("🔌 Client actors started - stdin/stdout bridge active");
    
    // Wait for Ctrl+C to shutdown
    tokio::signal::ctrl_c().await?;
    
    info!("Client bridge shutting down");
    Ok(())
}
