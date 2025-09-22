// Reference actor - handles storage and retrieval of symposium references

use anyhow::bail;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// Messages that can be sent to the reference actor
#[derive(Debug)]
pub enum ReferenceMessage {
    /// Store a reference with arbitrary JSON context
    StoreReference {
        key: String,
        value: Value,
        reply_tx: oneshot::Sender<anyhow::Result<()>>,
    },
    /// Retrieve a stored reference
    GetReference {
        key: String,
        reply_tx: oneshot::Sender<Option<Value>>,
    },
}

/// Actor that manages reference storage using a local HashMap
struct ReferenceActor {
    receiver: mpsc::Receiver<ReferenceMessage>,
    storage: HashMap<String, Value>,
}

impl ReferenceActor {
    fn new(receiver: mpsc::Receiver<ReferenceMessage>) -> Self {
        Self {
            receiver,
            storage: HashMap::new(),
        }
    }

    async fn run(mut self) {
        debug!("Reference actor started");
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
        debug!("Reference actor stopped");
    }

    async fn handle_message(&mut self, msg: ReferenceMessage) {
        match msg {
            ReferenceMessage::StoreReference {
                key,
                value,
                reply_tx,
            } => {
                debug!("Storing reference: {}", key);
                self.storage.insert(key, value);
                let _ = reply_tx.send(Ok(()));
            }
            ReferenceMessage::GetReference { key, reply_tx } => {
                debug!("Retrieving reference: {}", key);
                let value = self.storage.get(&key).cloned();
                let _ = reply_tx.send(value);
            }
        }
    }
}

/// Handle for communicating with the reference actor
#[derive(Clone)]
pub struct ReferenceHandle {
    sender: mpsc::Sender<ReferenceMessage>,
}

/// The result value. It's important that this has `{}`
/// because serde serializes that to a `{}` object which
/// is "truthy".
#[derive(Serialize, Debug)]
pub struct ReferenceStored {}

impl ReferenceHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = ReferenceActor::new(receiver);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    /// Store a reference with arbitrary JSON context
    pub async fn store_reference(&self, key: String, value: Value) -> anyhow::Result<ReferenceStored> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = ReferenceMessage::StoreReference {
            key,
            value,
            reply_tx,
        };

        if let Err(_) = self.sender.send(msg).await {
            bail!("Reference actor unavailable");
        }

        reply_rx.await??;

        Ok(ReferenceStored {})
    }

    /// Retrieve a stored reference
    pub async fn get_reference(&self, key: &str) -> Option<Value> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = ReferenceMessage::GetReference {
            key: key.to_string(),
            reply_tx,
        };

        if let Err(_) = self.sender.send(msg).await {
            error!("Failed to send get_reference message to actor");
            return None;
        }

        reply_rx.await.unwrap_or(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_store_and_retrieve_reference() {
        let handle = ReferenceHandle::new();

        let test_data = json!({
            "relativePath": "src/test.rs",
            "selectedText": "fn test() {}",
            "type": "code_selection"
        });

        // Store reference
        let result = handle
            .store_reference("test-uuid".to_string(), test_data.clone())
            .await;
        assert!(result.is_ok());

        // Retrieve reference
        let retrieved = handle.get_reference("test-uuid").await;
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_get_nonexistent_reference() {
        let handle = ReferenceHandle::new();

        let result = handle.get_reference("nonexistent").await;
        assert_eq!(result, None);
    }
}
