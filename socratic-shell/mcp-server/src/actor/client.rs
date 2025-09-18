//! IPC Client Actor - Transport layer for daemon communication
//!
//! Handles Unix socket connection management, message serialization/deserialization,
//! and forwards parsed IPCMessages via tokio channels.

use std::process::Command;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};
use anyhow::Result;
use crate::types::IPCMessage;

/// Requests that can be sent to the client actor
pub enum ClientRequest {
    /// Connect to daemon (with auto-start if needed)
    Connect {
        socket_prefix: String,
        auto_start: bool,
        reply_tx: oneshot::Sender<Result<()>>,
    },
    /// Send a message to daemon
    SendMessage {
        message: IPCMessage,
        reply_tx: oneshot::Sender<Result<()>>,
    },
    /// Disconnect from daemon
    Disconnect,
}

/// Actor that manages daemon connection and message transport
pub struct ClientActor {
    request_rx: mpsc::Receiver<ClientRequest>,
    /// Channel to send parsed messages from daemon
    outbound_tx: mpsc::Sender<IPCMessage>,
    /// Current connection state
    connection: Option<UnixStream>,
}

impl ClientActor {
    pub fn new(
        request_rx: mpsc::Receiver<ClientRequest>,
        outbound_tx: mpsc::Sender<IPCMessage>,
    ) -> Self {
        Self {
            request_rx,
            outbound_tx,
            connection: None,
        }
    }

    pub async fn run(mut self) {
        while let Some(request) = self.request_rx.recv().await {
            match request {
                ClientRequest::Connect { socket_prefix, auto_start, reply_tx } => {
                    let result = self.connect(&socket_prefix, auto_start).await;
                    let _ = reply_tx.send(result);
                }
                ClientRequest::SendMessage { message, reply_tx } => {
                    let result = self.send_message(message).await;
                    let _ = reply_tx.send(result);
                }
                ClientRequest::Disconnect => {
                    self.disconnect().await;
                    break;
                }
            }
        }
    }

    async fn connect(&mut self, socket_prefix: &str, auto_start: bool) -> Result<()> {
        let socket_path = crate::constants::daemon_socket_path(socket_prefix);

        // Try to connect to existing daemon
        let stream = match UnixStream::connect(&socket_path).await {
            Ok(stream) => {
                info!("✅ Connected to existing daemon at {}", socket_path);
                stream
            }
            Err(_) if auto_start => {
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

        // Start reading from daemon in background
        let (read_half, write_half) = stream.into_split();
        self.connection = Some(UnixStream::from_std(
            std::os::unix::net::UnixStream::pair()?.0
        )?);
        
        // Spawn reader task
        let outbound_tx = self.outbound_tx.clone();
        tokio::spawn(async move {
            Self::read_daemon_messages(read_half, outbound_tx).await;
        });

        // Store write half for sending messages
        // TODO: Store write_half properly for message sending
        
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

    async fn read_daemon_messages(
        read_half: tokio::net::unix::OwnedReadHalf,
        outbound_tx: mpsc::Sender<IPCMessage>,
    ) {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("Daemon connection closed");
                    break;
                }
                Ok(_) => {
                    let message_str = line.trim();
                    if !message_str.is_empty() {
                        match serde_json::from_str::<IPCMessage>(message_str) {
                            Ok(message) => {
                                if let Err(e) = outbound_tx.send(message).await {
                                    error!("Failed to forward message from daemon: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse message from daemon: {} - {}", e, message_str);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from daemon: {}", e);
                    break;
                }
            }
        }
    }

    async fn send_message(&mut self, message: IPCMessage) -> Result<()> {
        // TODO: Implement message sending to daemon
        // Need to store write_half properly in connect()
        warn!("Message sending not yet implemented: {:?}", message);
        Ok(())
    }

    async fn disconnect(&mut self) {
        if let Some(_connection) = self.connection.take() {
            info!("Disconnecting from daemon");
            // Connection will be dropped automatically
        }
    }
}

/// Handle for communicating with the client actor
#[derive(Clone)]
pub struct ClientHandle {
    sender: mpsc::Sender<ClientRequest>,
}

impl ClientHandle {
    pub fn new(outbound_tx: mpsc::Sender<IPCMessage>) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = ClientActor::new(receiver, outbound_tx);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    /// Connect to daemon with optional auto-start
    pub async fn connect(&self, socket_prefix: String, auto_start: bool) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let request = ClientRequest::Connect {
            socket_prefix,
            auto_start,
            reply_tx,
        };
        
        self.sender.send(request).await
            .map_err(|e| anyhow::anyhow!("Failed to send connect request: {}", e))?;
        
        reply_rx.await
            .map_err(|e| anyhow::anyhow!("Connect request cancelled: {}", e))?
    }

    /// Send a message to daemon
    pub async fn send_message(&self, message: IPCMessage) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let request = ClientRequest::SendMessage { message, reply_tx };
        
        self.sender.send(request).await
            .map_err(|e| anyhow::anyhow!("Failed to send message request: {}", e))?;
        
        reply_rx.await
            .map_err(|e| anyhow::anyhow!("Send message request cancelled: {}", e))?
    }

    /// Disconnect from daemon
    pub async fn disconnect(&self) -> Result<()> {
        self.sender.send(ClientRequest::Disconnect).await
            .map_err(|e| anyhow::anyhow!("Failed to send disconnect request: {}", e))
    }
}
