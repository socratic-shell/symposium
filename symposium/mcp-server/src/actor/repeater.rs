//! Repeater actor for centralized message routing and logging
//!
//! The repeater actor receives messages from clients and broadcasts them to all subscribers.
//! It maintains a central log of all messages for debugging purposes.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};

/// Maximum number of messages to keep in history
const MAX_MESSAGE_HISTORY: usize = 1024;

/// Messages sent to the repeater actor
#[derive(Debug)]
pub enum RepeaterMessage {
    /// Subscribe to receive broadcast messages
    Subscribe(mpsc::UnboundedSender<String>),
    /// Incoming message from a client to be broadcast
    IncomingMessage { from_client_id: usize, content: String },
    /// Request debug dump of message history
    DebugDump(oneshot::Sender<Vec<LoggedMessage>>),
    /// Set identifier for a client for debugging
    DebugSetIdentifier { client_id: usize, identifier: String },
}

/// A logged message with metadata
#[derive(Debug, Clone)]
pub struct LoggedMessage {
    pub timestamp: u64,
    pub from_client_id: usize,
    pub from_identifier: String,
    pub content: String,
}

/// The repeater actor that handles message routing and logging
struct RepeaterActor {
    /// List of subscribers to broadcast messages to
    subscribers: Vec<mpsc::UnboundedSender<String>>,
    /// History of broadcast messages for debugging
    message_history: VecDeque<LoggedMessage>,
    /// Client identifiers for debugging
    client_identifiers: HashMap<usize, String>,
}

impl RepeaterActor {
    /// Create a new repeater actor
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
            message_history: VecDeque::with_capacity(MAX_MESSAGE_HISTORY),
            client_identifiers: HashMap::new(),
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
                RepeaterMessage::DebugSetIdentifier { client_id, identifier } => {
                    self.client_identifiers.insert(client_id, identifier.clone());
                    info!("Set identifier for client {}: {}", client_id, identifier);
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

        let from_identifier = self.client_identifiers
            .get(&from_client_id)
            .cloned()
            .unwrap_or_else(|| from_client_id.to_string());

        let logged_message = LoggedMessage {
            timestamp,
            from_client_id,
            from_identifier: from_identifier.clone(),
            content: content.clone(),
        };

        // Add to history
        if self.message_history.len() >= MAX_MESSAGE_HISTORY {
            self.message_history.pop_front();
        }
        self.message_history.push_back(logged_message);

        // Check if this is a log message and skip broadcasting if so
        let mut is_log = false;
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                if msg_type == "log" {
                    // Don't broadcast log messages to avoid loops and noise
                    is_log = true;
                }
            }
        }

        // For anything other than a log message, broadcast to all subscribers, removing closed channels
        if !is_log {
            self.subscribers.retain(|sender| {
                match sender.send(content.clone()) {
                    Ok(_) => true,
                    Err(_) => {
                        // Channel is closed, remove this subscriber
                        false
                    }
                }
            });
        }

        info!("Broadcast message from client {} ({}) to {} subscribers", from_client_id, from_identifier, self.subscribers.len());
    }
}

/// Spawn a repeater actor task and return the sender for communicating with it
pub async fn spawn_repeater_task() -> mpsc::UnboundedSender<RepeaterMessage> {
    let (repeater_tx, repeater_rx) = mpsc::unbounded_channel::<RepeaterMessage>();
    let repeater_actor = RepeaterActor::new();
    tokio::spawn(repeater_actor.run(repeater_rx));
    repeater_tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_basic_message_routing() {
        let tx = spawn_repeater_task().await;
        
        // Create two subscribers
        let (sub1_tx, mut sub1_rx) = mpsc::unbounded_channel();
        let (sub2_tx, mut sub2_rx) = mpsc::unbounded_channel();
        
        // Subscribe both
        tx.send(RepeaterMessage::Subscribe(sub1_tx)).unwrap();
        tx.send(RepeaterMessage::Subscribe(sub2_tx)).unwrap();
        
        // Send a message
        tx.send(RepeaterMessage::IncomingMessage {
            from_client_id: 1,
            content: "test message".to_string(),
        }).unwrap();
        
        // Both subscribers should receive it
        let msg1 = timeout(Duration::from_millis(100), sub1_rx.recv()).await.unwrap().unwrap();
        let msg2 = timeout(Duration::from_millis(100), sub2_rx.recv()).await.unwrap().unwrap();
        
        assert_eq!(msg1, "test message");
        assert_eq!(msg2, "test message");
    }

    #[tokio::test]
    async fn test_client_identifiers() {
        let tx = spawn_repeater_task().await;
        
        // Set identifier for client 1
        tx.send(RepeaterMessage::DebugSetIdentifier {
            client_id: 1,
            identifier: "MCP-Server-123".to_string(),
        }).unwrap();
        
        // Send message from client 1
        tx.send(RepeaterMessage::IncomingMessage {
            from_client_id: 1,
            content: "hello".to_string(),
        }).unwrap();
        
        // Request debug dump
        let (dump_tx, dump_rx) = oneshot::channel();
        tx.send(RepeaterMessage::DebugDump(dump_tx)).unwrap();
        
        let history = timeout(Duration::from_millis(100), dump_rx).await.unwrap().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from_identifier, "MCP-Server-123");
        assert_eq!(history[0].content, "hello");
    }

    #[tokio::test]
    async fn test_closed_channel_cleanup() {
        let tx = spawn_repeater_task().await;
        
        // Create subscriber and then drop it
        let (sub_tx, sub_rx) = mpsc::unbounded_channel();
        tx.send(RepeaterMessage::Subscribe(sub_tx)).unwrap();
        drop(sub_rx); // Close the receiver
        
        // Send a message - should not panic and should clean up the closed channel
        tx.send(RepeaterMessage::IncomingMessage {
            from_client_id: 1,
            content: "test".to_string(),
        }).unwrap();
        
        // Give it time to process
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Test passes if no panic occurred
    }

    #[tokio::test]
    async fn test_message_history_limit() {
        let tx = spawn_repeater_task().await;
        
        // Send more than MAX_MESSAGE_HISTORY messages
        for i in 0..MAX_MESSAGE_HISTORY + 10 {
            tx.send(RepeaterMessage::IncomingMessage {
                from_client_id: 1,
                content: format!("message {}", i),
            }).unwrap();
        }
        
        // Request debug dump
        let (dump_tx, dump_rx) = oneshot::channel();
        tx.send(RepeaterMessage::DebugDump(dump_tx)).unwrap();
        
        let history = timeout(Duration::from_millis(100), dump_rx).await.unwrap().unwrap();
        
        // Should be limited to MAX_MESSAGE_HISTORY
        assert_eq!(history.len(), MAX_MESSAGE_HISTORY);
        
        // Should contain the most recent messages
        assert!(history.last().unwrap().content.contains(&format!("{}", MAX_MESSAGE_HISTORY + 9)));
    }
}
