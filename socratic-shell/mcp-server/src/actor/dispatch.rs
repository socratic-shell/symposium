//! IPC Dispatch Actor - Message router for IPC communication
//!
//! This actor handles message routing, reply correlation, and timeout management.
//! Extracted from the monolithic IPCCommunicator to provide focused responsibility.

use crate::actor::Actor;
use crate::types::{IPCMessage, IpcPayload};
use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use uuid;

/// Mock actor function type - takes incoming and outgoing channels
pub type MockActorFn = Box<
    dyn Fn(
            mpsc::Receiver<IPCMessage>,
            mpsc::Sender<IPCMessage>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

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

    /// Handle to Marco actor for discovery messages
    marco_handle: Option<crate::actor::MarcoHandle>,

    /// Handle to Reference actor for storing/retrieving context
    reference_handle: Option<crate::actor::ReferenceHandle>,

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
                                        if let Some(marco) = &self.marco_handle {
                                            if let Err(e) = marco.handle_marco(message, self.client_tx.clone()).await {
                                                tracing::error!("Failed to route Marco message: {}", e);
                                            }
                                        } else {
                                            tracing::debug!("Received Marco message but no Marco actor available");
                                        }
                                    }
                                    crate::types::IPCMessageType::StoreReference => {
                                        if let Some(reference) = &self.reference_handle {
                                            if let Err(e) = self.handle_store_reference(message, reference).await {
                                                tracing::error!("Failed to handle StoreReference message: {}", e);
                                            }
                                        } else {
                                            tracing::debug!("Received StoreReference message but no Reference actor available");
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
            self.pending_replies
                .retain(|_id, reply_tx| !reply_tx.is_closed());
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
        marco_handle: Option<crate::actor::MarcoHandle>,
        reference_handle: Option<crate::actor::ReferenceHandle>,
    ) -> Self {
        Self {
            request_rx,
            client_rx,
            client_tx,
            marco_handle,
            reference_handle,
            pending_replies: HashMap::new(),
        }
    }

    /// Handle StoreReference messages by routing to the reference actor
    async fn handle_store_reference(
        &self,
        message: IPCMessage,
        reference_handle: &crate::actor::ReferenceHandle,
    ) -> Result<(), String> {
        // Deserialize the StoreReference payload
        let payload: crate::types::StoreReferencePayload = serde_json::from_value(message.payload)
            .map_err(|e| format!("Failed to deserialize StoreReference payload: {}", e))?;

        // Store the reference using the reference actor
        reference_handle
            .store_reference(payload.key, payload.value)
            .await
            .map_err(|e| format!("Failed to store reference: {}", e))?;

        tracing::debug!("Successfully stored reference via reference actor");
        Ok(())
    }
}

/// Handle for communicating with the dispatch actor
#[derive(Clone)]
pub struct DispatchHandle {
    sender: mpsc::Sender<DispatchRequest>,
    shell_pid: u32,
    taskspace_uuid: Option<String>,
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
        shell_pid: u32,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(32);

        // Try to extract taskspace UUID from directory structure
        let taskspace_uuid = crate::ipc::extract_project_info().map(|(_, uuid)| uuid).ok();

        // Create Marco actor for discovery messages
        let marco_handle = crate::actor::MarcoHandle::new(shell_pid);

        // Create Reference actor for storing context data
        let reference_handle = crate::actor::ReferenceHandle::new(reference_store);

        let actor = DispatchActor::new(receiver, client_rx, client_tx, Some(marco_handle), Some(reference_handle));
        actor.spawn();

        Self { sender, shell_pid, taskspace_uuid }
    }

    /// Spawn a dispatch actor with a mock actor for testing
    pub fn spawn_with_mock(mock_fn: MockActorFn) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let (client_tx, client_rx) = mpsc::channel(32);
        let (mock_tx, mock_rx) = mpsc::channel(32);

        // Spawn the mock actor
        tokio::spawn(mock_fn(client_rx, mock_tx));

        // Create Marco actor for discovery messages
        let marco_handle = crate::actor::MarcoHandle::new(0);

        let actor = DispatchActor::new(receiver, mock_rx, client_tx, Some(marco_handle), None);
        actor.spawn();

        Self { sender, shell_pid: 0, taskspace_uuid: None }
    }

    /// Send a message out into the ether and (optionally) await a response.
    pub async fn send<M>(&self, message: M) -> anyhow::Result<M::Reply>
    where
        M: IpcPayload,
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
            taskspace_uuid: self.taskspace_uuid.clone(),
            shell_pid: Some(self.shell_pid),
        }
    }
}
