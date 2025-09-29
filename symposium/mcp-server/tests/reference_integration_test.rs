use serde_json::json;
use symposium_mcp::actor::{DispatchHandle, ReferenceHandle};
use symposium_mcp::types::{IPCMessage, IPCMessageType, MessageSender, StoreReferencePayload};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_reference_integration_store_and_retrieve() {
    // Create a reference handle (this is what the server would create)
    let reference_handle = ReferenceHandle::new();

    // Test data to store
    let test_data = json!({
        "relativePath": "src/auth.rs",
        "selectedText": "fn authenticate(token: &str) -> bool { ... }",
        "selectionRange": {
            "start": {"line": 42, "column": 0},
            "end": {"line": 45, "column": 1}
        },
        "type": "code_selection"
    });

    // 1. Simulate storing a reference via IPC (like VSCode extension would do)
    let store_result = reference_handle
        .store_reference("test-ref-uuid".to_string(), test_data.clone())
        .await;

    assert!(
        store_result.is_ok(),
        "Failed to store reference: {:?}",
        store_result
    );

    // 2. Simulate retrieving the reference (like expand_reference MCP tool would do)
    let retrieved_data = reference_handle.get_reference("test-ref-uuid").await;

    assert_eq!(
        retrieved_data,
        Some(test_data),
        "Retrieved data doesn't match stored data"
    );

    // 3. Test retrieving non-existent reference
    let missing_data = reference_handle.get_reference("nonexistent-uuid").await;
    assert_eq!(
        missing_data, None,
        "Should return None for non-existent reference"
    );
}

#[tokio::test]
async fn test_reference_integration_via_dispatch_actor() {
    // Create channels for mock IPC communication
    let (client_tx, mut client_rx) = mpsc::channel(32);
    let (mock_tx, mock_rx) = mpsc::channel(32);

    // Create reference handle
    let reference_handle = ReferenceHandle::new();

    // Create dispatch handle with the reference handle
    let _dispatch_handle = DispatchHandle::new(mock_rx, client_tx, Some(12345), reference_handle.clone());

    // Test data
    let test_context = json!({
        "filePath": "README.md",
        "type": "documentation",
        "lastModified": "2024-09-18T15:48:00Z"
    });

    // Create StoreReference IPC message (like VSCode extension sends)
    let store_payload = StoreReferencePayload {
        key: "integration-test-uuid".to_string(),
        value: test_context.clone(),
    };

    let ipc_message = IPCMessage {
        id: "msg-123".to_string(),
        message_type: IPCMessageType::StoreReference,
        payload: serde_json::to_value(store_payload).unwrap(),
        sender: MessageSender {
            working_directory: "/test/workspace".to_string(),
            taskspace_uuid: Some("test-taskspace-uuid".to_string()),
            shell_pid: Some(12345),
        },
    };

    // Send the message through the mock channel (simulating daemon → dispatch)
    mock_tx.send(ipc_message).await.unwrap();

    // Wait for the reply message (confirms storage completed)
    let reply = client_rx
        .recv()
        .await
        .expect("Should receive reply message");
    assert_eq!(reply.id, "msg-123");
    assert_eq!(reply.message_type, IPCMessageType::Response);

    // Verify the reply indicates success
    let reply_data: serde_json::Value = reply.payload;
    assert_eq!(reply_data["success"], true);

    // Now retrieve the reference directly via the handle (like expand_reference would)
    let retrieved = reference_handle
        .get_reference("integration-test-uuid")
        .await;

    assert_eq!(
        retrieved,
        Some(test_context),
        "Integration test: stored via IPC, retrieved via handle"
    );
}
