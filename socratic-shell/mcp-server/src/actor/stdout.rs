//! Stdout Actor - Prints IPCMessages to stdout for CLI mode
//!
//! Simple actor that receives IPCMessages from client actor and prints them
//! as JSON to stdout. Used when running in daemon client mode.

use tokio::io::{self, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{error, info};
use crate::types::IPCMessage;

/// Actor that prints IPCMessages to stdout
pub struct StdoutActor {
    /// Channel for receiving messages to print
    message_rx: mpsc::Receiver<IPCMessage>,
}

impl StdoutActor {
    pub fn new(message_rx: mpsc::Receiver<IPCMessage>) -> Self {
        Self { message_rx }
    }

    pub async fn run(mut self) {
        let mut stdout = io::stdout();
        
        while let Some(message) = self.message_rx.recv().await {
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
        
        info!("StdoutActor shutting down");
    }
}

/// Handle for communicating with the stdout actor
#[derive(Clone)]
pub struct StdoutHandle {
    sender: mpsc::Sender<IPCMessage>,
}

impl StdoutHandle {
    pub fn new() -> (Self, mpsc::Sender<IPCMessage>) {
        let (sender, receiver) = mpsc::channel(32);
        let actor = StdoutActor::new(receiver);
        tokio::spawn(async move { actor.run().await });

        // Return both the handle and the sender for wiring to ClientActor
        (Self { sender: sender.clone() }, sender)
    }
}
