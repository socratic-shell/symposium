// Daemon actor - handles communication with the daemon process

use tokio::sync::{mpsc, oneshot};
use crate::types::IPCMessage;

/// Requests that can be sent to the daemon actor
pub enum DaemonRequest {
    /// Send a message on the IPC channel and optionally ask for a reply
    SendMessage {
        /// Message to send
        message: IPCMessage,
        /// If Some, then this is a channel on which the sender expects a reply
        reply_tx: Option<oneshot::Sender<serde_json::Value>>,
    },
    /// Sender stopped waiting for a reply for `id` due to timeout
    CancelReply { id: String },
}

/// Actor that manages daemon communication
struct DaemonActor {
    receiver: mpsc::Receiver<DaemonRequest>,
    // TODO: Add daemon connection state and pending replies map
}

impl DaemonActor {
    fn new(receiver: mpsc::Receiver<DaemonRequest>) -> Self {
        Self { receiver }
    }

    async fn run(mut self) {
        while let Some(request) = self.receiver.recv().await {
            self.handle_request(request).await;
        }
    }

    async fn handle_request(&mut self, request: DaemonRequest) {
        match request {
            DaemonRequest::SendMessage { message, reply_tx } => {
                // TODO: Implement message sending to daemon
            }
            DaemonRequest::CancelReply { id } => {
                // TODO: Implement reply cancellation
            }
        }
    }
}

/// Handle for communicating with the daemon actor
#[derive(Clone)]
pub struct DaemonHandle {
    sender: mpsc::Sender<DaemonRequest>,
}

impl DaemonHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32); // Larger buffer for daemon communication
        let actor = DaemonActor::new(receiver);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    /// Send a message without expecting a reply
    pub async fn send_message(&self, message: IPCMessage) -> Result<(), mpsc::error::SendError<DaemonRequest>> {
        let request = DaemonRequest::SendMessage {
            message,
            reply_tx: None,
        };
        self.sender.send(request).await
    }

    /// Send a message and wait for a reply
    pub async fn send_message_with_reply(&self, message: IPCMessage) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let request = DaemonRequest::SendMessage {
            message,
            reply_tx: Some(reply_tx),
        };
        
        self.sender.send(request).await?;
        let reply = reply_rx.await?;
        Ok(reply)
    }

    // TODO: Add other daemon communication methods
}
