use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    use uuid;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = ReferenceStore::new();
        
        let test_value = serde_json::json!({
            "file": "src/main.rs",
            "line": 42,
            "selection": "let x = foo();",
            "custom_field": "arbitrary data"
        });

        let id = uuid::Uuid::new_v4().to_string();
        store.store_json_with_id(&id, test_value.clone()).await.unwrap();
        let retrieved = store.get_json(&id).await.unwrap().unwrap();
        
        assert_eq!(retrieved["file"], "src/main.rs");
        assert_eq!(retrieved["line"], 42);
        assert_eq!(retrieved["selection"], "let x = foo();");
        assert_eq!(retrieved["custom_field"], "arbitrary data");
    }

    #[tokio::test]
    async fn test_store_json_with_id() {
        let store = ReferenceStore::new();
        
        let test_value = serde_json::json!({
            "file": "test.rs",
            "user_comment": "Test comment",
            "custom_data": 42
        });

        let id = "test-id";
        store.store_json_with_id(id, test_value.clone()).await.unwrap();
        
        let retrieved = store.get_json(id).await.unwrap().unwrap();
        assert_eq!(retrieved["user_comment"], "Test comment");
        assert_eq!(retrieved["custom_data"], 42);
    }

    #[tokio::test]
    async fn test_count() {
        let store = ReferenceStore::new();
        
        let test_value1 = serde_json::json!({"type": "test1"});
        let test_value2 = serde_json::json!({"type": "test2"});

        assert_eq!(store.count().await, 0);
        
        store.store_json_with_id("id1", test_value1).await.unwrap();
        store.store_json_with_id("id2", test_value2).await.unwrap();
        
        assert_eq!(store.count().await, 2);
    }
}
