//! Stdio Actor - Bridges stdin/stdout for CLI mode
//!
//! Reads from stdin and sends to daemon via ClientActor, receives IPCMessages
//! from ClientActor and prints to stdout. Used in daemon client mode.

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{error, info};
use crate::actor::Actor;
use crate::types::IPCMessage;

/// Actor that bridges stdin/stdout with daemon communication
pub struct StdioActor {
    /// Channel for receiving messages to print to stdout
    message_rx: mpsc::Receiver<IPCMessage>,
    /// Channel for sending messages from stdin to daemon
    outbound_tx: mpsc::Sender<IPCMessage>,
}

impl Actor for StdioActor {
    async fn run(mut self) {
        let mut stdout = io::stdout();
        let stdin = io::stdin();
        let mut stdin_reader = BufReader::new(stdin);
        let mut stdin_line = String::new();

        loop {
            tokio::select! {
                // Read from stdin, parse and send to daemon
                result = stdin_reader.read_line(&mut stdin_line) => {
                    match result {
                        Ok(0) => {
                            info!("Stdin closed");
                            break;
                        }
                        Ok(_) => {
                            let line = stdin_line.trim();
                            if !line.is_empty() {
                                match serde_json::from_str::<IPCMessage>(line) {
                                    Ok(message) => {
                                        if let Err(e) = self.outbound_tx.send(message).await {
                                            error!("Failed to send message to daemon: {}", e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to parse stdin message: {} - {}", e, line);
                                    }
                                }
                            }
                            stdin_line.clear();
                        }
                        Err(e) => {
                            error!("Error reading from stdin: {}", e);
                            break;
                        }
                    }
                }

                // Receive messages from daemon and print to stdout
                message = self.message_rx.recv() => {
                    match message {
                        Some(message) => {
                            match serde_json::to_string(&message) {
                                Ok(json) => {
                                    let line = format!("{}\n", json);
                                    if let Err(e) = stdout.write_all(line.as_bytes()).await {
                                        error!("Failed to write to stdout: {}", e);
                                        break;
                                    }
                                    if let Err(e) = stdout.flush().await {
                                        error!("Failed to flush stdout: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to serialize message to JSON: {}", e);
                                }
                            }
                        }
                        None => {
                            info!("Message channel closed");
                            break;
                        }
                    }
                }
            }
        }

        info!("StdioActor shutting down");
    }
}

impl StdioActor {
    pub fn new(
        message_rx: mpsc::Receiver<IPCMessage>,
        outbound_tx: mpsc::Sender<IPCMessage>,
    ) -> Self {
        Self {
            message_rx,
            outbound_tx,
        }
    }
}

/// Handle for communicating with the stdio actor
#[derive(Clone)]
pub struct StdioHandle {
    // For future use if needed
}

impl StdioHandle {
    pub fn new(outbound_tx: mpsc::Sender<IPCMessage>) -> (Self, mpsc::Sender<IPCMessage>) {
        let (inbound_tx, inbound_rx) = mpsc::channel(32);
        let actor = StdioActor::new(inbound_rx, outbound_tx);
        actor.spawn();

        // Return handle and the sender for ClientActor to send messages to stdio
        (Self {}, inbound_tx)
    }
}
