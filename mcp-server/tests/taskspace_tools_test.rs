//! Basic tests for taskspace orchestration tools
//!
//! These tests verify that the new types and structures compile correctly
//! and can be serialized/deserialized properly.

use serde_json;
use symposium_mcp::types::*;

#[test]
fn test_spawn_taskspace_payload_serialization() {
    let payload = SpawnTaskspacePayload {
        name: "test-taskspace".to_string(),
        task_description: "Test task description".to_string(),
        initial_prompt: "Test initial prompt".to_string(),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: SpawnTaskspacePayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.name, deserialized.name);
    assert_eq!(payload.task_description, deserialized.task_description);
    assert_eq!(payload.initial_prompt, deserialized.initial_prompt);
}

#[test]
fn test_log_progress_payload_serialization() {
    let payload = LogProgressPayload {
        message: "Test progress message".to_string(),
        category: ProgressCategory::Milestone,
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: LogProgressPayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(payload.message, deserialized.message);
    assert!(matches!(deserialized.category, ProgressCategory::Milestone));
}

#[test]
fn test_signal_user_payload_serialization() {
    let payload = SignalUserPayload {
        message: "Need assistance with this task".to_string(),
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&payload).expect("Failed to serialize");
    let deserialized: SignalUserPayload = serde_json::from_str(&json).expect("Failed to deserialize");
    
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
fn test_ipc_message_types_include_taskspace_operations() {
    // Test that the new message types can be serialized
    let spawn_type = IPCMessageType::SpawnTaskspace;
    let log_type = IPCMessageType::LogProgress;
    let signal_type = IPCMessageType::SignalUser;

    // Should serialize without errors
    assert!(serde_json::to_string(&spawn_type).is_ok());
    assert!(serde_json::to_string(&log_type).is_ok());
    assert!(serde_json::to_string(&signal_type).is_ok());
}
