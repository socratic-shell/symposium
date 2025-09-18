//! Marco Actor - Handles discovery protocol by responding to Marco with Polo
//!
//! When VSCode extension broadcasts "Marco?" (who's out there?), this actor responds with "Polo!" (I'm here!)

use crate::actor::Actor;
use crate::types::{IPCMessage, PoloMessage};
use tokio::sync::mpsc;
use tracing::{error, info};

/// Message types that the Marco actor can handle
#[derive(Debug)]
pub enum MarcoMessage {
    /// Handle incoming Marco discovery message and respond with Polo
    HandleMarco { 
        message: IPCMessage,
        response_tx: mpsc::Sender<IPCMessage>,
    },
}

/// Actor that handles marco discovery protocol by responding with polo
pub struct MarcoActor {
    /// Channel for receiving marco messages
    message_rx: mpsc::Receiver<MarcoMessage>,
    /// Shell PID for this MCP server instance
    shell_pid: u32,
}

impl Actor for MarcoActor {
    async fn run(mut self) {
        info!("Marco actor started for shell PID {}", self.shell_pid);

        while let Some(message) = self.message_rx.recv().await {
            match message {
                MarcoMessage::HandleMarco { message, response_tx } => {
                    info!("Received Marco discovery message, responding with Polo");
                    
                    // Create Polo response
                    let polo_response = IPCMessage {
                        message_type: crate::types::IPCMessageType::Polo,
                        id: uuid::Uuid::new_v4().to_string(),
                        sender: message.sender.clone(), // Use same sender context
                        payload: serde_json::to_value(PoloMessage {
                            terminal_shell_pid: self.shell_pid,
                        }).unwrap_or_default(),
                    };

                    if let Err(e) = response_tx.send(polo_response).await {
                        error!("Failed to send Polo response: {}", e);
                    }
                }
            }
        }

        info!("Marco actor shutting down");
    }
}

impl MarcoActor {
    /// Create a new Marco actor
    fn new(message_rx: mpsc::Receiver<MarcoMessage>, shell_pid: u32) -> Self {
        Self {
            message_rx,
            shell_pid,
        }
    }
}

/// Handle for communicating with the Marco actor
#[derive(Clone)]
pub struct MarcoHandle {
    sender: mpsc::Sender<MarcoMessage>,
}

impl MarcoHandle {
    /// Spawn a new Marco actor and return a handle
    pub fn new(shell_pid: u32) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = MarcoActor::new(receiver, shell_pid);
        actor.spawn();

        Self { sender }
    }

    /// Send a marco message to be handled (will respond with polo)
    pub async fn handle_marco(
        &self,
        message: IPCMessage,
        response_tx: mpsc::Sender<IPCMessage>,
    ) -> Result<(), mpsc::error::SendError<MarcoMessage>> {
        self.sender
            .send(MarcoMessage::HandleMarco { message, response_tx })
            .await
    }
}
