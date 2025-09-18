// Reference actor - handles storage and retrieval of socratic shell references

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// Messages that can be sent to the reference actor
#[derive(Debug)]
pub enum ReferenceMessage {
    /// Store a reference with arbitrary JSON context
    StoreReference {
        key: String,
        value: Value,
        reply_tx: oneshot::Sender<Result<(), String>>,
    },
    /// Retrieve a stored reference
    GetReference {
        key: String,
        reply_tx: oneshot::Sender<Option<Value>>,
    },
}

/// Actor that manages reference storage using the persistent ReferenceStore
struct ReferenceActor {
    receiver: mpsc::Receiver<ReferenceMessage>,
    reference_store: Arc<crate::reference_store::ReferenceStore>,
}

impl ReferenceActor {
    fn new(
        receiver: mpsc::Receiver<ReferenceMessage>,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Self {
        Self {
            receiver,
            reference_store,
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
            ReferenceMessage::StoreReference { key, value, reply_tx } => {
                debug!("Storing reference: {}", key);
                match self.reference_store.store_json_with_id(&key, value).await {
                    Ok(()) => {
                        let _ = reply_tx.send(Ok(()));
                    }
                    Err(e) => {
                        error!("Failed to store reference {}: {}", key, e);
                        let _ = reply_tx.send(Err(e.to_string()));
                    }
                }
            }
            ReferenceMessage::GetReference { key, reply_tx } => {
                debug!("Retrieving reference: {}", key);
                match self.reference_store.get_json(&key).await {
                    Ok(value) => {
                        let _ = reply_tx.send(value);
                    }
                    Err(e) => {
                        error!("Failed to retrieve reference {}: {}", key, e);
                        let _ = reply_tx.send(None);
                    }
                }
            }
        }
    }
}

/// Handle for communicating with the reference actor
#[derive(Clone)]
pub struct ReferenceHandle {
    sender: mpsc::Sender<ReferenceMessage>,
}

impl ReferenceHandle {
    pub fn new(reference_store: Arc<crate::reference_store::ReferenceStore>) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = ReferenceActor::new(receiver, reference_store);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    /// Store a reference with arbitrary JSON context
    pub async fn store_reference(&self, key: String, value: Value) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = ReferenceMessage::StoreReference { key, value, reply_tx };
        
        if let Err(_) = self.sender.send(msg).await {
            return Err("Reference actor unavailable".to_string());
        }
        
        reply_rx.await.unwrap_or_else(|_| Err("Reference actor response failed".to_string()))
    }

    /// Retrieve a stored reference
    pub async fn get_reference(&self, key: &str) -> Option<Value> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = ReferenceMessage::GetReference { 
            key: key.to_string(), 
            reply_tx 
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
        let reference_store = Arc::new(crate::reference_store::ReferenceStore::new());
        let handle = ReferenceHandle::new(reference_store);
        
        let test_data = json!({
            "relativePath": "src/test.rs",
            "selectedText": "fn test() {}",
            "type": "code_selection"
        });
        
        // Store reference
        let result = handle.store_reference("test-uuid".to_string(), test_data.clone()).await;
        assert!(result.is_ok());
        
        // Retrieve reference
        let retrieved = handle.get_reference("test-uuid").await;
        assert_eq!(retrieved, Some(test_data));
    }

    #[tokio::test]
    async fn test_get_nonexistent_reference() {
        let reference_store = Arc::new(crate::reference_store::ReferenceStore::new());
        let handle = ReferenceHandle::new(reference_store);
        
        let result = handle.get_reference("nonexistent").await;
        assert_eq!(result, None);
    }
}
