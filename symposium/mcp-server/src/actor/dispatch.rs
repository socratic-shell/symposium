//! IPC Dispatch Actor - Message router for IPC communication
//!
//! This actor handles message routing, reply correlation, and timeout management.
//! Extracted from the monolithic IPCCommunicator to provide focused responsibility.

use crate::actor::Actor;
use crate::types::{IPCMessage, IpcPayload, MessageSender, PoloPayload, ResponsePayload};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
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
    reply_tx: Option<oneshot::Sender<ResponsePayload>>,
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

    /// Identity when sending messages
    sender: MessageSender,

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
    pending_replies: HashMap<String, oneshot::Sender<ResponsePayload>>,
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
                        Some(message) => self.handle_incoming_message(message).await,
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
        sender: MessageSender,
        reference_handle: Option<crate::actor::ReferenceHandle>,
    ) -> Self {
        Self {
            request_rx,
            client_rx,
            client_tx,
            sender,
            reference_handle,
            pending_replies: HashMap::new(),
        }
    }

    async fn handle_incoming_message(&mut self, message: IPCMessage) {
        match message.message_type {
            crate::types::IPCMessageType::Marco => {
                tracing::debug!("Received `marco` message with id `{id}` from {sender:?}",
                    id = message.id,
                    sender = message.sender,
                );

                // Marco we just handle right here. It's so simple it's not worth factoring out.
                if let Err(e) = self.send_polo().await {
                    tracing::error!("Failed to route Marco message: {}", e);
                }
            }
            crate::types::IPCMessageType::Response => {
                if let Some(reply_tx) = self.pending_replies.remove(&message.id) {
                    tracing::debug!("Received `response` to message with id `{id}` from {sender:?}",
                        id = message.id,
                        sender = message.sender,
                    );

                    // This is a reply to a pending request.
                    match ResponsePayload::deserialize(&message.payload) {
                        Ok(payload) => {
                            if let Err(e) = reply_tx.send(payload) {
                                // Ignore send errors - the listener may have timed out and closed the channel
                                tracing::debug!("Could not forward response: {e:?}");
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Error parsing reply as a `ResponsePayload`: error={e:?}, payload={:?}", message.payload);
                            return;
                        }
                    }
                }
            }
            crate::types::IPCMessageType::StoreReference => {
                tracing::debug!("Received `store_reference` with id `{id}` from {sender:?}",
                    id = message.id,
                    sender = message.sender,
                );

                if let Some(reference) = &self.reference_handle {
                    if let Err(e) = self.handle_store_reference(message, reference).await {
                        tracing::error!("Failed to handle StoreReference message: {}", e);
                    }
                } else {
                    tracing::debug!(
                        "No Reference actor available"
                    );
                }
            }
            _ => {
                // Ignore other messages, not relevant to us
            }
        }
    }

    /// Handle StoreReference messages by routing to the reference actor
    async fn handle_store_reference(
        &self,
        message: IPCMessage,
        reference_handle: &crate::actor::ReferenceHandle,
    ) -> anyhow::Result<()> {
        // Deserialize the StoreReference payload
        let payload: crate::types::StoreReferencePayload = serde_json::from_value(message.payload)
            .with_context(|| format!("failed to deserialize StoreReference payload"))?;

        tracing::debug!("forwarding to reference actor: {payload:?}");

        // Store the reference using the reference actor
        let result = reference_handle
            .store_reference(payload.key, payload.value)
            .await;

        self.respond_to(&message.id, result).await
    }

    async fn respond_to<T: Serialize + std::fmt::Debug>(
        &self,
        incoming_message_id: &String,
        data: Result<T, impl Display>,
    ) -> anyhow::Result<()> {
        let payload = match data {
            Ok(data) => ResponsePayload {
                success: true,
                error: None,
                data: Some(data),
            },
            Err(err) => ResponsePayload {
                success: false,
                error: Some(err.to_string()),
                data: None,
            },
        };

        debug!("responding to message with id {incoming_message_id:?} with {payload:?}");

        let reply = IPCMessage {
            id: incoming_message_id.clone(), // Same ID for correlation
            message_type: crate::types::IPCMessageType::Response, // Always use Response type for replies
            payload: serde_json::to_value(payload)?,
            sender: self.sender.clone(),
        };

        Ok(self.client_tx.send(reply).await?)
    }
    
    async fn send_polo(&self) -> anyhow::Result<()> {
        tracing::debug!("responding with polo message");
        let ipc_message = IPCMessage {
            message_type: crate::types::IPCMessageType::Polo,
            id: fresh_message_id(),
            sender: self.sender.clone(),
            payload: serde_json::to_value(PoloPayload {})?
        };
        Ok(self.client_tx.send(ipc_message).await?)
    }
}

/// Handle for communicating with the dispatch actor
#[derive(Clone)]
pub struct DispatchHandle {
    /// Send messages to the dispatch actor
    actor_tx: mpsc::Sender<DispatchRequest>,

    /// Identity when sending messages
    sender: MessageSender,
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
        shell_pid: Option<u32>,
        reference_handle: crate::actor::ReferenceHandle,
    ) -> Self {
        let (actor_tx, actor_rx) = mpsc::channel(32);

        let sender = create_sender(shell_pid);
        info!("MCP server sender with PID {shell_pid:?} sender info: {sender:?}");

        let actor = DispatchActor::new(
            actor_rx,
            client_rx,
            client_tx,
            sender.clone(),
            Some(reference_handle),
        );
        actor.spawn();

        Self { actor_tx, sender }
    }

    /// Spawn a dispatch actor with a mock actor for testing
    pub fn spawn_with_mock(mock_fn: MockActorFn) -> Self {
        let (actor_tx, actor_rx) = mpsc::channel(32);
        let (client_tx, client_rx) = mpsc::channel(32);
        let (mock_tx, mock_rx) = mpsc::channel(32);

        // Spawn the mock actor
        tokio::spawn(mock_fn(client_rx, mock_tx));

        let sender = MessageSender {
            working_directory: working_directory(),
            taskspace_uuid: None,
            shell_pid: None,
        };

        let actor = DispatchActor::new(
            actor_rx,
            mock_rx,
            client_tx,
            sender.clone(),
            None,
        );
        actor.spawn();

        Self { actor_tx, sender }
    }

    /// Send a message out into the ether and (optionally) await a response.
    pub async fn send<M>(&self, message: M) -> anyhow::Result<M::Reply>
    where
        M: IpcPayload,
    {
        let id = fresh_message_id();
        let message_type = message.message_type();
        let payload = serde_json::to_value(message)?;
        let message = IPCMessage {
            message_type,
            id: id.clone(),
            payload,
            sender: self.sender.clone(),
        };

        let (reply_tx, reply_rx) = if M::EXPECTS_REPLY {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        self.actor_tx
            .send(DispatchRequest { message, reply_tx })
            .await?;

        match reply_rx {
            Some(reply_rx) => tokio::select! {
                response = reply_rx => {
                    // Extract data from ResponsePayload wrapper
                    let response = response?;
                    if !response.success {
                        return Err(anyhow::anyhow!("Request failed: {}", response.error.unwrap_or_default()));
                    }
                    
                    // For unit type replies, data can be None
                    let data = response.data.unwrap_or(serde_json::Value::Null);
                    Ok(<M::Reply>::deserialize(data)?)
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    return Err(anyhow::anyhow!("Request timed out after 30 seconds"));
                }
            },

            None => Ok(serde_json::from_value(serde_json::Value::Null)?),
        }
    }

}

fn create_sender(shell_pid: Option<u32>) -> crate::types::MessageSender {
    // Try to extract taskspace UUID from directory structure
    let taskspace_uuid = crate::ipc::extract_project_info()
        .map(|(_, uuid)| uuid)
        .ok();
    crate::types::MessageSender {
        working_directory: working_directory(),
        taskspace_uuid,
        shell_pid,
    }
}

fn working_directory() -> String {
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("/"))
        .to_string_lossy()
        .to_string()
}

    fn fresh_message_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }