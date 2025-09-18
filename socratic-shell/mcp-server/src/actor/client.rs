//! IPC Client Actor - Transport layer for daemon communication
//!
//! Handles Unix socket connection management, message serialization/deserialization,
//! and forwards parsed IPCMessages via tokio channels.

use std::process::Command;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use anyhow::Result;
use crate::types::IPCMessage;

/// Actor that manages daemon connection and message transport
pub struct ClientActor {
    /// Channel to receive messages to send to daemon
    inbound_rx: mpsc::Receiver<IPCMessage>,
    /// Channel to send parsed messages from daemon
    outbound_tx: mpsc::Sender<IPCMessage>,
    /// Socket configuration
    socket_prefix: String,
    auto_start: bool,
}

impl ClientActor {
    pub fn new(
        inbound_rx: mpsc::Receiver<IPCMessage>,
        outbound_tx: mpsc::Sender<IPCMessage>,
        socket_prefix: String,
        auto_start: bool,
    ) -> Self {
        Self {
            inbound_rx,
            outbound_tx,
            socket_prefix,
            auto_start,
        }
    }

    pub async fn run(mut self) {
        loop {
            // Check if outbound channel is closed
            if self.outbound_tx.is_closed() {
                info!("Outbound channel closed, shutting down client actor");
                break;
            }

            match self.connect_and_run().await {
                Ok(()) => {
                    info!("Client actor completed normally");
                    break;
                }
                Err(e) => {
                    error!("Client actor error: {}", e);
                    // TODO: Add reconnection logic with backoff
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn connect_and_run(&mut self) -> Result<()> {
        let socket_path = crate::constants::daemon_socket_path(&self.socket_prefix);

        // Try to connect to existing daemon
        let stream = match UnixStream::connect(&socket_path).await {
            Ok(stream) => {
                info!("✅ Connected to existing daemon at {}", socket_path);
                stream
            }
            Err(_) if self.auto_start => {
                info!("No daemon found, attempting to start one...");
                self.spawn_daemon().await?;
                self.wait_for_daemon(&socket_path).await?
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to connect to daemon at {}: {}",
                    socket_path,
                    e
                ));
            }
        };

        // Split stream for reading and writing
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        loop {
            tokio::select! {
                // Read from daemon and forward to outbound channel
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => {
                            info!("Daemon connection closed");
                            break;
                        }
                        Ok(_) => {
                            let message_str = line.trim();
                            if !message_str.is_empty() {
                                match serde_json::from_str::<IPCMessage>(message_str) {
                                    Ok(message) => {
                                        if let Err(e) = self.outbound_tx.send(message).await {
                                            error!("Failed to forward message from daemon: {}", e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse message from daemon: {} - {}", e, message_str);
                                    }
                                }
                            }
                            line.clear();
                        }
                        Err(e) => {
                            error!("Error reading from daemon: {}", e);
                            break;
                        }
                    }
                }

                // Receive messages to send to daemon
                message = self.inbound_rx.recv() => {
                    match message {
                        Some(message) => {
                            match serde_json::to_string(&message) {
                                Ok(json) => {
                                    let line = format!("{}\n", json);
                                    if let Err(e) = write_half.write_all(line.as_bytes()).await {
                                        error!("Failed to write to daemon: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to serialize message: {}", e);
                                }
                            }
                        }
                        None => {
                            info!("Inbound channel closed");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn spawn_daemon(&self) -> Result<()> {
        let current_exe = std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("Failed to get current executable: {}", e))?;

        let mut cmd = Command::new(&current_exe);
        cmd.args(&["daemon"]);

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child = cmd
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

        info!("Spawned daemon process (PID: {})", child.id());
        Ok(())
    }

    async fn wait_for_daemon(&self, socket_path: &str) -> Result<UnixStream> {
        let mut attempts = 0;
        loop {
            if attempts >= 20 {
                return Err(anyhow::anyhow!("Timeout waiting for daemon to start"));
            }

            match UnixStream::connect(socket_path).await {
                Ok(stream) => {
                    info!("✅ Connected to newly started daemon");
                    return Ok(stream);
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    attempts += 1;
                }
            }
        }
    }
}

/// Handle for communicating with the client actor
#[derive(Clone)]
pub struct ClientHandle {
    sender: mpsc::Sender<IPCMessage>,
}

impl ClientHandle {
    pub fn new(
        socket_prefix: String,
        auto_start: bool,
    ) -> (Self, mpsc::Sender<IPCMessage>) {
        let (inbound_tx, inbound_rx) = mpsc::channel(32);
        let (outbound_tx, outbound_rx) = mpsc::channel(32);
        
        let actor = ClientActor::new(inbound_rx, outbound_tx.clone(), socket_prefix, auto_start);
        tokio::spawn(async move { actor.run().await });

        // Return handle and the receiver for other actors to get messages from daemon
        (Self { sender: inbound_tx }, outbound_tx)
    }

    /// Send a message to daemon
    pub async fn send_message(&self, message: IPCMessage) -> Result<()> {
        self.sender.send(message).await
            .map_err(|e| anyhow::anyhow!("Failed to send message to daemon: {}", e))
    }
}
