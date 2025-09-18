//! IPC Dispatch Actor - Message router for IPC communication
//!
//! This actor handles message routing, reply correlation, and timeout management.
//! Extracted from the monolithic IPCCommunicator to provide focused responsibility.

use crate::types::IPCMessage;
use crate::{actor::Actor, types::IPCMessageType};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use uuid;

/// Send a message on the IPC channel and optionally ask for a reply.
struct DispatchRequest {
    /// Message to send.
    message: IPCMessage,

    /// If `Some`, then this is a channel on which the
    /// sender expects a reply. We will wait for a reply
    /// to `message.id` and then send the value.
    reply_tx: Option<oneshot::Sender<serde_json::Value>>,
}

/// A [Tokio actor][] that shepherds the connection to the daemon.
/// This actor owns the mutable state storing the pending replies.
///
/// [Tokio actor]: https://ryhl.io/blog/actors-with-tokio/
struct DispatchActor {
    /// Channel for receiving actor requests.
    ///
    /// Actor terminates when this channel is closed.
    request_rx: mpsc::Receiver<DispatchRequest>,

    /// Incoming messages from the IPC client
    client_rx: mpsc::Receiver<IPCMessage>,

    /// Outgoing messages to the IPC client.
    client_tx: mpsc::Sender<IPCMessage>,

    /// Handle to MarcoPolo actor for discovery messages
    marco_polo_handle: Option<crate::actor::MarcoPoloHandle>,

    /// Map whose key is the `id` of a reply that we are expecting
    /// and the value is the channel where we should send it when it arrives.
    ///
    /// When the listener times out, they will send us a [`DispatchRequest::CancelReply`][]
    /// message. When we receive it, we'll remove the entry from this map.
    /// But if the reply arrives before we get that notification, we may find
    /// that the Sender in this map is closed when we send the data along.
    /// That's ok.
    pending_replies: HashMap<String, oneshot::Sender<serde_json::Value>>,
}

impl Actor for DispatchActor {
    async fn run(mut self) {
        loop {
            // Main dispatch loop: handle incoming requests and client messages
            // - DispatchRequest: outgoing messages that may expect replies
            // - IPCMessage: incoming messages from client (replies or unsolicited)
            tokio::select! {
                // Handle outgoing message requests
                request = self.request_rx.recv() => {
                    match request {
                        Some(DispatchRequest { message, reply_tx }) => {
                            // Store reply channel if expecting a response
                            if let Some(reply_tx) = reply_tx {
                                self.pending_replies.insert(message.id.clone(), reply_tx);
                            }
                            
                            // Send message to client
                            if let Err(e) = self.client_tx.send(message).await {
                                tracing::error!("Failed to send message to client: {}", e);
                                break;
                            }
                        }
                        None => {
                            tracing::info!("Request channel closed, shutting down dispatch actor");
                            break;
                        }
                    }
                }

                // Handle incoming messages from client
                message = self.client_rx.recv() => {
                    match message {
                        Some(message) => {
                            // Try to match against pending replies
                            if let Some(reply_tx) = self.pending_replies.remove(&message.id) {
                                // This is a reply to a pending request
                                // Ignore send errors - the listener may have timed out and closed the channel
                                let _ = reply_tx.send(message.payload);
                            } else {
                                // Unsolicited message - route to appropriate actor
                                match message.message_type {
                                    crate::types::IPCMessageType::Marco => {
                                        if let Some(marco_polo) = &self.marco_polo_handle {
                                            if let Err(e) = marco_polo.handle_marco(message).await {
                                                tracing::error!("Failed to route Marco message: {}", e);
                                            }
                                        } else {
                                            tracing::debug!("Received Marco message but no MarcoPolo actor available");
                                        }
                                    }
                                    crate::types::IPCMessageType::Polo => {
                                        if let Some(marco_polo) = &self.marco_polo_handle {
                                            if let Err(e) = marco_polo.handle_polo(message).await {
                                                tracing::error!("Failed to route Polo message: {}", e);
                                            }
                                        } else {
                                            tracing::debug!("Received Polo message but no MarcoPolo actor available");
                                        }
                                    }
                                    _ => {
                                        tracing::debug!("Received unsolicited message: {:?}", message.message_type);
                                    }
                                }
                            }
                        }
                        None => {
                            tracing::info!("Client channel closed, shutting down dispatch actor");
                            break;
                        }
                    }
                }
            }

            // Clean up any closed reply channels (timed out requests)
            self.pending_replies.retain(|_id, reply_tx| !reply_tx.is_closed());
        }
    }
}

impl DispatchActor {
    /// Create a new dispatch actor and wire-up to other actors
    ///
    /// * A "client" that can send/receive `IPCMessage` values. This is the underlying transport.
    /// * Other actors that should receive particular types of incoming messages (e.g., Marco/Polo messages).
    fn new(
        request_rx: mpsc::Receiver<DispatchRequest>,
        client_rx: mpsc::Receiver<IPCMessage>,
        client_tx: mpsc::Sender<IPCMessage>,
        marco_polo_handle: Option<crate::actor::MarcoPoloHandle>,
    ) -> Self {
        Self {
            request_rx,
            client_rx,
            client_tx,
            marco_polo_handle,
            pending_replies: HashMap::new(),
        }
    }
}

/// Handle for communicating with the dispatch actor
#[derive(Clone)]
pub struct DispatchHandle {
    sender: mpsc::Sender<DispatchRequest>,
}

impl DispatchHandle {
    /// Spawn a new dispatch actor and return a handle for interacting with it.
    ///
    /// Spawning a dispatch actor requires providing various interconnections:
    ///
    /// * A "client" that can send/receive `IPCMessage` values. This is the underlying transport.
    /// * Other actors that should receive particular types of incoming messages (e.g., Marco/Polo messages).
    pub fn new(
        client_rx: mpsc::Receiver<IPCMessage>,
        client_tx: mpsc::Sender<IPCMessage>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        
        // Create MarcoPolo actor for discovery messages
        // TODO: Get shell PID from context
        let marco_polo_handle = crate::actor::MarcoPoloHandle::new(0);
        
        let actor = DispatchActor::new(receiver, client_rx, client_tx, Some(marco_polo_handle));
        actor.spawn();

        Self { sender }
    }

    /// Send a message out into the ether and (optionally) await a response.
    pub async fn send<M>(&self, message: M) -> anyhow::Result<M::Reply>
    where
        M: DispatchMessage,
    {
        let id = self.fresh_message_id();
        let message_type = message.message_type();
        let payload = serde_json::to_value(message)?;
        let message = IPCMessage {
            message_type,
            id: id.clone(),
            payload,
            sender: self.create_sender(),
        };

        let (reply_tx, reply_rx) = if M::EXPECTS_REPLY {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        self.sender
            .send(DispatchRequest { message, reply_tx })
            .await?;

        let reply_payload = match reply_rx {
            Some(reply_rx) => tokio::select! {
                result = reply_rx => {
                    result?
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    return Err(anyhow::anyhow!("Request timed out after 30 seconds"));
                }
            },

            None => serde_json::Value::Null,
        };

        Ok(<M::Reply>::deserialize(reply_payload)?)
    }

    fn fresh_message_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn create_sender(&self) -> crate::types::MessageSender {
        crate::types::MessageSender {
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            taskspace_uuid: None, // TODO: Get from context if available
            shell_pid: None,      // TODO: Get from context if available
        }
    }
}

/// Trait implemented by messages that can be sent over the dispatch handle.
pub trait DispatchMessage: Serialize {
    /// If true, we should wait for a reply after sending this message.
    /// If false, just return `()`.
    const EXPECTS_REPLY: bool;

    /// Type of reply expected; this is `()` if no reply is expected.
    type Reply: DeserializeOwned;

    /// Value of `type` field in [`IPCMessage`].
    fn message_type(&self) -> IPCMessageType;
}
