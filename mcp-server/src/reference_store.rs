use anyhow::Result;
use crate::types::ReferenceContext;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory reference storage
#[derive(Debug, Clone)]
pub struct ReferenceStore {
    references: Arc<RwLock<HashMap<String, Value>>>,
}

impl ReferenceStore {
    /// Create a new reference store
    pub fn new() -> Self {
        Self {
            references: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a reference context and return a unique ID
    pub async fn store(&self, context: ReferenceContext) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.store_with_id(&id, context).await?;
        Ok(id)
    }

    /// Store a reference context with a specific ID
    pub async fn store_with_id(&self, id: &str, context: ReferenceContext) -> Result<()> {
        let mut refs = self.references.write().await;
        let value = serde_json::to_value(context)?;
        refs.insert(id.to_string(), value);
        Ok(())
    }

    /// Store arbitrary JSON value with a specific ID (for generic reference system)
    pub async fn store_json_with_id(&self, id: &str, value: serde_json::Value) -> Result<()> {
        let mut refs = self.references.write().await;
        refs.insert(id.to_string(), value);
        Ok(())
    }

    /// Retrieve arbitrary JSON value by ID (for generic reference system)
    pub async fn get_json(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let refs = self.references.read().await;
        Ok(refs.get(id).cloned())
    }

    /// Retrieve a reference context by ID
    pub async fn get(&self, id: &str) -> Result<Option<ReferenceContext>> {
        let refs = self.references.read().await;
        match refs.get(id) {
            Some(value) => {
                let context: ReferenceContext = serde_json::from_value(value.clone())?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    /// Get the number of stored references
    pub async fn count(&self) -> usize {
        let refs = self.references.read().await;
        refs.len()
    }
}

impl Default for ReferenceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = ReferenceStore::new();
        
        let context = ReferenceContext {
            file: Some("src/main.rs".to_string()),
            line: Some(42),
            selection: Some("let x = foo();".to_string()),
            user_comment: None,
            metadata: HashMap::new(),
        };

        let id = store.store(context.clone()).await.unwrap();
        let retrieved = store.get(&id).await.unwrap().unwrap();
        
        assert_eq!(retrieved.file, context.file);
        assert_eq!(retrieved.line, context.line);
        assert_eq!(retrieved.selection, context.selection);
    }

    #[tokio::test]
    async fn test_store_with_id() {
        let store = ReferenceStore::new();
        
        let context = ReferenceContext {
            file: Some("test.rs".to_string()),
            line: None,
            selection: None,
            user_comment: Some("Test comment".to_string()),
            metadata: HashMap::new(),
        };

        let id = "test-id";
        store.store_with_id(id, context.clone()).await.unwrap();
        
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_comment, context.user_comment);
    }

    #[tokio::test]
    async fn test_count() {
        let store = ReferenceStore::new();
        
        let context = ReferenceContext {
            file: Some("test.rs".to_string()),
            line: None,
            selection: None,
            user_comment: None,
            metadata: HashMap::new(),
        };

        assert_eq!(store.count().await, 0);
        
        store.store(context.clone()).await.unwrap();
        store.store(context.clone()).await.unwrap();
        
        assert_eq!(store.count().await, 2);
    }
}
