//! IPC Dispatch Actor - Message router for IPC communication
//!
//! This actor handles message routing, reply correlation, and timeout management.
//! Extracted from the monolithic IPCCommunicator to provide focused responsibility.

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use crate::actor::Actor;
use crate::types::IPCMessage;

/// Requests from the rest of the system over the IPC dispatch actor.
pub enum DispatchRequest {
    /// Send a message on the IPC channel and optionally ask for a reply.
    SendMessage {
        /// Message to send.
        message: IPCMessage,

        /// If `Some`, then this is a channel on which the
        /// sender expects a reply. We will wait for a reply
        /// to `message.id` and then send the value.
        reply_tx: Option<oneshot::Sender<serde_json::Value>>,
    },

    /// Sender stopped waiting for a reply for `id` due to timeout.
    CancelReply { id: String },
}

/// A [Tokio actor][] that shepherds the connection to the daemon.
/// This actor owns the mutable state storing the pending replies.
///
/// [Tokio actor]: https://ryhl.io/blog/actors-with-tokio/
pub struct DispatchActor {
    /// Channel for receiving actor requests.
    ///
    /// Actor terminates when this channel is closed.
    request_rx: mpsc::Receiver<DispatchRequest>,

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
        // The logic here is
        //
        // 10. If not connected to the daemon, connect. I would kind of like it if
        //     we did this by invoking `daemon::run_client` except
        //     that this function is hardcoded presently to use stdio and to do serialization,
        //     it'd be be nice if there was an inner code
        //     we could reuse that relayed messages via tokio pipes.
        //
        //     While connecting, if `self.request_rx` is closed, goto 99.
        //
        // 20. Once connected, await requests on `self.request_rx`; incoming messages from
        //     the daemon; `self.request_rx` being closed; daemon disconnecting.
        //
        // 30. If `self.request_rx` is closed, goto 99. If daemon disconnected, goto 10.
        //
        // 40. If a `DispatchRequest` arrived, process it:
        //     - send the message over the daemon if relevant, store the reply channel in a local hashtable
        //     - if cancel-reply, remove reply channel from local hashtable, if present
        //
        // 50. If a daemon message arrived:
        //     - if it is a reply, check if we have a registered channel, send to that channel, ignore errors.
        //     - otherwise, if it is a `Marco`, respond with `Polo`
        //     - otherwise, ... Claude: are there other messages we care about? let's discuss ...
        //     - otherwise, ignore.
        //
        // 99. Return. This should drop any connection to the daemon we may have and cause it to exit.

        while let Some(request) = self.request_rx.recv().await {
            self.handle_request(request).await;
        }
    }
}

impl DispatchActor {
    pub fn new(request_rx: mpsc::Receiver<DispatchRequest>) -> Self {
        Self {
            request_rx,
            pending_replies: HashMap::new(),
        }
    }

    async fn handle_request(&mut self, request: DispatchRequest) {
        match request {
            DispatchRequest::SendMessage { message, reply_tx } => {
                if let Some(reply_tx) = reply_tx {
                    self.pending_replies.insert(message.id.clone(), reply_tx);
                }
                // TODO: Forward message to server/client actors
            }
            DispatchRequest::CancelReply { id } => {
                self.pending_replies.remove(&id);
            }
        }
    }
}

/// Handle for communicating with the dispatch actor
#[derive(Clone)]
pub struct DispatchHandle {
    sender: mpsc::Sender<DispatchRequest>,
}

impl DispatchHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = DispatchActor::new(receiver);
        actor.spawn();

        Self { sender }
    }

    /// Send a message without expecting a reply
    pub async fn send_message(&self, message: IPCMessage) -> Result<(), mpsc::error::SendError<DispatchRequest>> {
        let request = DispatchRequest::SendMessage {
            message,
            reply_tx: None,
        };
        self.sender.send(request).await
    }

    /// Send a message and wait for a reply with timeout
    pub async fn send_message_with_reply<R>(&self, message: IPCMessage) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        R: serde::de::DeserializeOwned,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        let message_id = message.id.clone();
        let request = DispatchRequest::SendMessage {
            message,
            reply_tx: Some(reply_tx),
        };
        
        self.sender.send(request).await?;
        
        // Wait for reply with timeout
        let reply = tokio::select! {
            result = reply_rx => {
                result?
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                // Cancel the pending reply on timeout
                let _ = self.cancel_reply(message_id).await;
                return Err("Request timed out after 30 seconds".into());
            }
        };
        
        // Deserialize to the requested type
        let deserialized = serde_json::from_value(reply)?;
        Ok(deserialized)
    }

    /// Cancel a pending reply
    pub async fn cancel_reply(&self, id: String) -> Result<(), mpsc::error::SendError<DispatchRequest>> {
        let request = DispatchRequest::CancelReply { id };
        self.sender.send(request).await
    }
}
