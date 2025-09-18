//! Marco/Polo Actor - Handles discovery message protocol
//!
//! This actor manages the marco/polo discovery protocol between MCP servers and VSCode extension.
//! Marco messages are broadcasts asking "who's out there?", Polo messages announce presence.

use crate::actor::Actor;
use crate::types::{IPCMessage, IPCMessageType, MarcoMessage, PoloMessage};
use tokio::sync::mpsc;
use tracing::{info, error};

/// Message types that the MarcoPolo actor can handle
#[derive(Debug)]
pub enum MarcoPoloMessage {
    /// Handle incoming Marco discovery message
    HandleMarco { message: IPCMessage },
    /// Handle incoming Polo discovery message  
    HandlePolo { message: IPCMessage },
}

/// Actor that handles marco/polo discovery protocol
pub struct MarcoPoloActor {
    /// Channel for receiving marco/polo messages
    message_rx: mpsc::Receiver<MarcoPoloMessage>,
    
    /// Shell PID for this MCP server instance
    shell_pid: u32,
}

impl Actor for MarcoPoloActor {
    async fn run(mut self) {
        info!("MarcoPolo actor started for shell PID {}", self.shell_pid);
        
        while let Some(message) = self.message_rx.recv().await {
            match message {
                MarcoPoloMessage::HandleMarco { message } => {
                    info!("Received Marco discovery message from {}", message.sender.working_directory);
                    // TODO: Respond with Polo message
                }
                MarcoPoloMessage::HandlePolo { message } => {
                    if let Ok(polo_msg) = serde_json::from_value::<PoloMessage>(message.payload) {
                        info!("Received Polo discovery message from shell PID {}", polo_msg.terminal_shell_pid);
                    } else {
                        error!("Failed to deserialize Polo message");
                    }
                }
            }
        }
        
        info!("MarcoPolo actor shutting down");
    }
}

impl MarcoPoloActor {
    /// Create a new MarcoPolo actor
    fn new(message_rx: mpsc::Receiver<MarcoPoloMessage>, shell_pid: u32) -> Self {
        Self {
            message_rx,
            shell_pid,
        }
    }
}

/// Handle for communicating with the MarcoPolo actor
#[derive(Clone)]
pub struct MarcoPoloHandle {
    sender: mpsc::Sender<MarcoPoloMessage>,
}

impl MarcoPoloHandle {
    /// Spawn a new MarcoPolo actor and return a handle
    pub fn new(shell_pid: u32) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = MarcoPoloActor::new(receiver, shell_pid);
        actor.spawn();

        Self { sender }
    }

    /// Send a marco message to be handled
    pub async fn handle_marco(&self, message: IPCMessage) -> Result<(), mpsc::error::SendError<MarcoPoloMessage>> {
        self.sender.send(MarcoPoloMessage::HandleMarco { message }).await
    }

    /// Send a polo message to be handled
    pub async fn handle_polo(&self, message: IPCMessage) -> Result<(), mpsc::error::SendError<MarcoPoloMessage>> {
        self.sender.send(MarcoPoloMessage::HandlePolo { message }).await
    }
}
