//! Repeater actor for centralized message routing and logging
//!
//! The repeater actor receives messages from clients and broadcasts them to all subscribers.
//! It maintains a central log of all messages for debugging purposes.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

/// Maximum number of messages to keep in history
const MAX_MESSAGE_HISTORY: usize = 32_000;

/// Messages sent to the repeater actor
#[derive(Debug)]
pub enum RepeaterMessage {
    /// Subscribe to receive broadcast messages
    Subscribe(mpsc::UnboundedSender<String>),
    /// Incoming message from a client to be broadcast
    IncomingMessage { from_client_id: usize, content: String },
    /// Request debug dump of message history
    DebugDump(oneshot::Sender<Vec<LoggedMessage>>),
}

/// A logged message with metadata
#[derive(Debug, Clone)]
pub struct LoggedMessage {
    pub timestamp: u64,
    pub from_client_id: usize,
    pub content: String,
}

/// The repeater actor that handles message routing and logging
pub struct RepeaterActor {
    /// List of subscribers to broadcast messages to
    subscribers: Vec<mpsc::UnboundedSender<String>>,
    /// History of broadcast messages for debugging
    message_history: VecDeque<LoggedMessage>,
}

impl RepeaterActor {
    /// Create a new repeater actor
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
            message_history: VecDeque::with_capacity(MAX_MESSAGE_HISTORY),
        }
    }

    /// Run the repeater actor, processing messages from the receiver
    pub async fn run(mut self, mut receiver: mpsc::UnboundedReceiver<RepeaterMessage>) {
        info!("Repeater actor started");

        while let Some(message) = receiver.recv().await {
            match message {
                RepeaterMessage::Subscribe(sender) => {
                    self.subscribers.push(sender);
                    info!("New subscriber added, total: {}", self.subscribers.len());
                }
                RepeaterMessage::IncomingMessage { from_client_id, content } => {
                    self.handle_incoming_message(from_client_id, content);
                }
                RepeaterMessage::DebugDump(response_sender) => {
                    let history = self.message_history.iter().cloned().collect();
                    if let Err(_) = response_sender.send(history) {
                        error!("Failed to send debug dump response");
                    }
                }
            }
        }

        info!("Repeater actor stopped");
    }

    /// Handle an incoming message by broadcasting it to all subscribers
    fn handle_incoming_message(&mut self, from_client_id: usize, content: String) {
        // Log the message
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let logged_message = LoggedMessage {
            timestamp,
            from_client_id,
            content: content.clone(),
        };

        // Add to history
        if self.message_history.len() >= MAX_MESSAGE_HISTORY {
            self.message_history.pop_front();
        }
        self.message_history.push_back(logged_message);

        // Broadcast to all subscribers, removing closed channels
        self.subscribers.retain(|sender| {
            match sender.send(content.clone()) {
                Ok(_) => true,
                Err(_) => {
                    // Channel is closed, remove this subscriber
                    false
                }
            }
        });

        info!("Broadcast message from client {} to {} subscribers", from_client_id, self.subscribers.len());
    }
}
