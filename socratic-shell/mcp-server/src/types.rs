//! Shared types for Dialectic MCP Server
//!
//! Mirrors the TypeScript types from server/src/types.ts to ensure
//! protocol compatibility across the IPC boundary.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actor::dispatch::DispatchMessage;

/// Parameters for the present-walkthrough MCP tool
///
/// Walkthroughs are markdown documents with embedded XML elements for interactive features
// ANCHOR: present_walkthrough_params
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct PresentWalkthroughParams {
    /// Markdown content with embedded XML elements (comment, gitdiff, action, mermaid)
    /// See dialectic guidance for XML element syntax and usage
    pub content: String,

    /// Base directory path for resolving relative file references
    #[serde(rename = "baseUri")]
    pub base_uri: String,
}
// ANCHOR_END: present_walkthrough_params

/// Parameters for log messages sent via IPC
// ANCHOR: log_params
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogParams {
    /// Log level
    pub level: LogLevel,

    /// Log message content
    pub message: String,
}
// ANCHOR_END: log_params

/// Log levels for IPC communication
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Error,
    Debug,
}

/// Marco discovery message - broadcasts "who's out there?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarcoMessage {
    // Marco messages have no payload
}

impl DispatchMessage for MarcoMessage {
    const EXPECTS_REPLY: bool = false;
    type Reply = ();

    fn message_type(&self) -> IPCMessageType {
        IPCMessageType::Marco
    }
}

/// Polo discovery message - announces presence with shell PID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoloMessage {
    #[serde(rename = "terminalShellPid")]
    pub terminal_shell_pid: u32,
}

impl DispatchMessage for PoloMessage {
    const EXPECTS_REPLY: bool = false;
    type Reply = ();

    fn message_type(&self) -> IPCMessageType {
        IPCMessageType::Polo
    }
}

/// Response from the get-selection tool
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetSelectionResult {
    /// Currently selected text, null if no selection
    #[serde(rename = "selectedText")]
    pub selected_text: Option<String>,

    /// File path of the active editor, if available
    #[serde(rename = "filePath")]
    pub file_path: Option<String>,

    /// Starting line number (1-based)
    #[serde(rename = "startLine")]
    pub start_line: Option<u32>,

    /// Starting column number (1-based)
    #[serde(rename = "startColumn")]
    pub start_column: Option<u32>,

    /// Ending line number (1-based)
    #[serde(rename = "endLine")]
    pub end_line: Option<u32>,

    /// Ending column number (1-based)
    #[serde(rename = "endColumn")]
    pub end_column: Option<u32>,

    /// Single line number if selection is on one line
    #[serde(rename = "lineNumber")]
    pub line_number: Option<u32>,

    /// Language ID of the document
    #[serde(rename = "documentLanguage")]
    pub document_language: Option<String>,

    /// Whether the document is untitled
    #[serde(rename = "isUntitled")]
    pub is_untitled: Option<bool>,

    /// Message explaining the selection state
    pub message: Option<String>,
}

/// Payload for Polo discovery messages (MCP server announces presence)
// ANCHOR: polo_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoloPayload {
    // Shell PID is now at top level in IPCMessage
}
// ANCHOR_END: polo_payload

/// Payload for Goodbye discovery messages (MCP server announces departure)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoodbyePayload {
    // Shell PID is now at top level in IPCMessage
}

/// Payload for ResolveSymbolByName messages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResolveSymbolByNamePayload {
    /// The symbol name to resolve (e.g., "User", "validateToken")
    pub name: String,
}

/// Payload for FindAllReferences messages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FindAllReferencesPayload {
    /// The resolved symbol to find references for
    pub symbol: crate::ide::SymbolDef,
}

/// Payload for Response messages (replaces IPCResponse struct)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponsePayload {
    /// Whether the operation succeeded
    pub success: bool,

    /// Optional error message
    pub error: Option<String>,

    /// Optional data payload for responses like get_selection
    pub data: Option<serde_json::Value>,
}

/// Sender information for message routing
// ANCHOR: message_sender
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageSender {
    /// Working directory - always present for reliable matching
    #[serde(rename = "workingDirectory")]
    pub working_directory: String,

    /// Optional taskspace UUID for taskspace-specific routing
    #[serde(rename = "taskspaceUuid")]
    pub taskspace_uuid: Option<String>,

    /// Optional shell PID - only when VSCode parent found
    #[serde(rename = "shellPid")]
    pub shell_pid: Option<u32>,
}
// ANCHOR_END: message_sender

/// IPC message sent from MCP server to VSCode extension
// ANCHOR: ipc_message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IPCMessage {
    /// Message type identifier
    #[serde(rename = "type")]
    pub message_type: IPCMessageType,

    /// Unique message ID for response tracking
    pub id: String,

    /// Sender information for routing
    pub sender: MessageSender,

    /// Message payload - for store_reference: { key: string, value: arbitrary_json }
    pub payload: serde_json::Value,
}
// ANCHOR_END: ipc_message

/// IPC message types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IPCMessageType {
    PresentReview,
    PresentWalkthrough,
    Log,
    GetSelection,
    /// Extension broadcasts "who's out there?" to discover active MCP servers
    Marco,
    /// MCP server announces presence with shell PID (response to Marco or unsolicited)
    Polo,
    /// MCP server announces departure with shell PID
    Goodbye,
    /// Response to any message (replaces IPCResponse struct)
    Response,
    /// Resolve symbol by name - returns Vec<ResolvedSymbol>
    ResolveSymbolByName,
    /// Find all references to a symbol - returns Vec<FileLocation>
    FindAllReferences,

    /// User feedback from VSCode extension (comments, review completion)
    UserFeedback,
    /// Store reference context for compact symposium-ref system
    StoreReference,
    /// Signal VSCode extension to reload window (sent by daemon on shutdown)
    ReloadWindow,
    /// Create new taskspace with initial prompt
    SpawnTaskspace,
    /// Report progress from agent with visual indicators
    LogProgress,
    /// Request user attention for assistance
    SignalUser,
    /// Update taskspace name and description
    UpdateTaskspace,
    /// Get/update taskspace state - unified operation that can both read and write
    TaskspaceState,
    /// Broadcast to discover active taskspaces for window registration
    TaskspaceRollCall,
    /// Register VSCode window with taskspace
    RegisterTaskspaceWindow,
    /// Delete current taskspace
    DeleteTaskspace,
}

// ANCHOR: store_reference_payload
/// Payload for store_reference messages - generic key-value storage
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StoreReferencePayload {
    /// UUID key for the reference
    pub key: String,
    /// Arbitrary JSON value - self-documenting structure determined by extension
    pub value: serde_json::Value,
}
// ANCHOR_END: store_reference_payload

/// Payload for user feedback messages from VSCode extension
// ANCHOR: user_feedback_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserFeedbackPayload {
    pub review_id: String,
    pub feedback_type: String, // "comment" or "complete_review"
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub comment_text: Option<String>,
    pub completion_action: Option<String>, // "request_changes", "checkpoint", "return"
    pub additional_notes: Option<String>,
    pub context_lines: Option<Vec<String>>,
}
// ANCHOR_END: user_feedback_payload

/// Parameters for presenting a review to the user
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresentReviewParams {
    pub content: String,
    pub mode: ReviewMode,
    pub section: Option<String>,
    pub base_uri: String,
}

/// Mode for presenting reviews
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ReviewMode {
    Replace,
    Append,
    UpdateSection,
}

/// Payload for spawn_taskspace messages
// ANCHOR: spawn_taskspace_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpawnTaskspacePayload {
    pub project_path: String,
    pub taskspace_uuid: String,
    pub name: String,
    pub task_description: String,
    pub initial_prompt: String,
}
// ANCHOR_END: spawn_taskspace_payload

/// Payload for log_progress messages
// ANCHOR: log_progress_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogProgressPayload {
    pub project_path: String,
    pub taskspace_uuid: String,
    pub message: String,
    pub category: ProgressCategory,
}
// ANCHOR_END: log_progress_payload

impl DispatchMessage for LogProgressPayload {
    const EXPECTS_REPLY: bool = false;

    type Reply = ();

    fn message_type(&self) -> IPCMessageType {
        IPCMessageType::LogProgress
    }
}

/// Progress categories for visual indicators
// ANCHOR: progress_category
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressCategory {
    Info,
    Warn,
    Error,
    Milestone,
    Question,
}
// ANCHOR_END: progress_category

/// Payload for signal_user messages
// ANCHOR: signal_user_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SignalUserPayload {
    pub project_path: String,
    pub taskspace_uuid: String,
    pub message: String,
}
// ANCHOR_END: signal_user_payload

/// Payload for update_taskspace messages
// ANCHOR: update_taskspace_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateTaskspacePayload {
    pub project_path: String,
    pub taskspace_uuid: String,
    pub name: String,
    pub description: String,
}
// ANCHOR_END: update_taskspace_payload

/// Unified payload for taskspace state operations (get/update)
///
/// This message type handles both reading and writing taskspace state.
/// - For read-only: Send with name=None, description=None  
/// - For update: Send with new name/description values
/// - Response: Always returns complete TaskspaceStateResponse
///
/// **Benefits of unified approach:**
/// - Single message type for all taskspace state operations
/// - GUI app can clear initial_prompt on any update operation
/// - Simpler protocol with consistent request/response pattern
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskspaceStateRequest {
    pub project_path: String,
    pub taskspace_uuid: String,
    /// New name to set (None = don't update)
    pub name: Option<String>,
    /// New description to set (None = don't update)  
    pub description: Option<String>,
}

/// Payload for get_taskspace_state messages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetTaskspaceStatePayload {
    pub project_path: String,
    pub taskspace_uuid: String,
}

/// Response for get_taskspace_state messages
///
/// This structure represents the complete state of a taskspace as managed by the
/// Symposium GUI application. It's used for dynamic agent initialization and
/// taskspace management.
///
/// **Field Usage:**
/// - `name`: User-visible taskspace name (shown in GUI, tabs, etc.)
/// - `description`: Short user-visible summary (shown in GUI, tooltips, etc.)  
/// - `initial_prompt`: Task description given to LLM during agent initialization
///
/// **Lifecycle:**
/// 1. GUI app creates taskspace with name, description, initial_prompt
/// 2. Agent requests state via get_taskspace_state → receives all fields
/// 3. Agent uses initial_prompt for initialization context
/// 4. Agent calls update_taskspace → GUI app returns same struct with initial_prompt=None
/// 5. This naturally clears the initial prompt after agent startup
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskspaceStateResponse {
    /// User-visible taskspace name (displayed in GUI)
    pub name: Option<String>,
    /// User-visible short summary (displayed in GUI)
    pub description: Option<String>,
    /// Task description for LLM initialization (cleared after agent startup)
    pub initial_prompt: Option<String>,
}

/// Payload for delete_taskspace messages
// ANCHOR: delete_taskspace_payload
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteTaskspacePayload {
    pub project_path: String,
    pub taskspace_uuid: String,
}
// ANCHOR_END: delete_taskspace_payload
