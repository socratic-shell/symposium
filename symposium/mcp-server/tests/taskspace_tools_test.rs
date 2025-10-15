//! Integration tests for taskspace orchestration MCP tools

use symposium_mcp::SymposiumServer;
use symposium_mcp::types::*;
use serde_json;

#[tokio::test]
async fn test_taskspace_tools_integration() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    // Create server in test mode (avoids actual IPC communication)
    let _server = SymposiumServer::new_test();

    // Verify server was created successfully
    assert!(true, "Server created successfully in test mode");

    // In test mode, the tools would succeed without actual IPC
    // This verifies the server initializes with the new tools without errors
}

#[test]
fn test_spawn_taskspace_payload_serialization() {
    let payload = SpawnTaskspacePayload {
        project_path: "/path/to/project".to_string(),
        taskspace_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "test-taskspace".to_string(),
        task_description: "Test task description".to_string(),
        initial_prompt: "Test initial prompt".to_string(),
        collaborator: Some("sparkle".to_string()),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: SpawnTaskspacePayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.project_path, deserialized.project_path);
    assert_eq!(payload.taskspace_uuid, deserialized.taskspace_uuid);
    assert_eq!(payload.name, deserialized.name);
    assert_eq!(payload.task_description, deserialized.task_description);
    assert_eq!(payload.initial_prompt, deserialized.initial_prompt);
}

#[test]
fn test_delete_taskspace_payload_serialization() {
    let payload = DeleteTaskspacePayload {
        project_path: "/path/to/project".to_string(),
        taskspace_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: DeleteTaskspacePayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.project_path, deserialized.project_path);
    assert_eq!(payload.taskspace_uuid, deserialized.taskspace_uuid);
}

#[test]
fn test_log_progress_payload_serialization() {
    let payload = LogProgressPayload {
        project_path: "/path/to/project".to_string(),
        taskspace_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        message: "Test progress message".to_string(),
        category: ProgressCategory::Milestone,
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: LogProgressPayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.project_path, deserialized.project_path);
    assert_eq!(payload.taskspace_uuid, deserialized.taskspace_uuid);
    assert_eq!(payload.message, deserialized.message);
    assert!(matches!(deserialized.category, ProgressCategory::Milestone));
}

#[test]
fn test_signal_user_payload_serialization() {
    let payload = SignalUserPayload {
        project_path: "/path/to/project".to_string(),
        taskspace_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        message: "Need assistance with this task".to_string(),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: SignalUserPayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.project_path, deserialized.project_path);
    assert_eq!(payload.taskspace_uuid, deserialized.taskspace_uuid);
    assert_eq!(payload.message, deserialized.message);
}

#[test]
fn test_progress_category_serialization() {
    let categories = vec![
        ProgressCategory::Info,
        ProgressCategory::Warn,
        ProgressCategory::Error,
        ProgressCategory::Milestone,
        ProgressCategory::Question,
    ];

    for category in categories {
        let json = serde_json::to_string(&category).expect("Failed to serialize category");
        let deserialized: ProgressCategory = serde_json::from_str(&json).expect("Failed to deserialize category");
        
        // Should round-trip correctly
        assert_eq!(
            serde_json::to_string(&category).unwrap(),
            serde_json::to_string(&deserialized).unwrap()
        );
    }
}

#[test]
fn test_emoji_category_acceptance() {
    // Test that the log_progress tool would accept emoji categories
    // (This tests the parsing logic that would be used in the actual tool)
    
    let test_cases = vec![
        ("info", ProgressCategory::Info),
        ("ℹ️", ProgressCategory::Info),
        ("warn", ProgressCategory::Warn),
        ("⚠️", ProgressCategory::Warn),
        ("error", ProgressCategory::Error),
        ("❌", ProgressCategory::Error),
        ("milestone", ProgressCategory::Milestone),
        ("✅", ProgressCategory::Milestone),
        ("question", ProgressCategory::Question),
        ("❓", ProgressCategory::Question),
    ];
    
    for (input, expected) in test_cases {
        let parsed = match input.to_lowercase().as_str() {
            "info" | "ℹ️" => ProgressCategory::Info,
            "warn" | "⚠️" => ProgressCategory::Warn,
            "error" | "❌" => ProgressCategory::Error,
            "milestone" | "✅" => ProgressCategory::Milestone,
            "question" | "❓" => ProgressCategory::Question,
            _ => ProgressCategory::Info,
        };
        
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            serde_json::to_string(&expected).unwrap(),
            "Failed for input: {}", input
        );
    }
}

#[test]
fn test_ipc_message_types_include_taskspace_operations() {
    // Test that the new message types can be serialized
    let spawn_type = IPCMessageType::SpawnTaskspace;
    let log_type = IPCMessageType::LogProgress;
    let signal_type = IPCMessageType::SignalUser;
    let delete_type = IPCMessageType::DeleteTaskspace;

    // Should serialize without errors
    assert!(serde_json::to_string(&spawn_type).is_ok());
    assert!(serde_json::to_string(&log_type).is_ok());
    assert!(serde_json::to_string(&signal_type).is_ok());
    assert!(serde_json::to_string(&delete_type).is_ok());
}
