//! IPC Dispatch Actor - Message router for IPC communication
//!
//! This actor handles message routing, reply correlation, and timeout management.
//! Extracted from the monolithic IPCCommunicator to provide focused responsibility.

use crate::types::IPCMessage;
use crate::{actor::Actor, types::IPCMessageType};
use anyhow::Context;
use chrono::serde::ts_microseconds::deserialize;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

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
            // XXX Claude: you need to do the following here
            //
            // 1. Wait for incoming messages on either request_rx or client_rx.
            // 2. If either of them gives you a `None` value (i.e., no more transmitters), then shutdown.
            // 3. Otherwise, process the incoming message:
            //    - if it is a DispatchRequest, insert the `reply_rx` into our `pending_replies` map and send it to the `client_tx`
            //    - if it is an incoming `IPCMessage`, either
            //        - attempt to match it against a reply
            //        - otherwise ignore it
            //        - later, we'll dispatch to marco/polo or other actors
            // 4. Also, use `self.pending_replies.retain` to drop any cases where the `reply_rx` is closed
            //    (means the listener timed out).
            //
            // Don't forget to write a nice comment explaining the logic.
        }
    }
}

impl DispatchActor {
    /// Create a new dispatch actor and write-up to other actors
    ///
    /// * A "client" that can send/receive `IPCMessage` values. This is the underlying transport.
    /// * Other actors that should receive particular types of incoming messages (e.g., Marco/Polo messages).
    fn new(request_rx: mpsc::Receiver<DispatchRequest>) -> Self {
        Self {
            request_rx,
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
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = DispatchActor::new(receiver);
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
        // XXX Claude -- fill this in with code to make a fresh UUID
    }

    fn create_sender(&self) -> MessageSender {
        // XXX Claude -- fill this in
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
