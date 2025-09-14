//! Dialectic MCP Server implementation using the official rmcp SDK
//!
//! Provides get_selection, ide_operation, and present_walkthrough tools for AI assistants
//! to interact with the VSCode extension via IPC.

use anyhow::Result;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use rust_embed::RustEmbed;
use serde_json;
use std::future::Future;
use std::sync::Arc;
use tracing::{info, warn};

use crate::dialect::DialectInterpreter;
use crate::ipc::IPCCommunicator;
use crate::reference_store::ReferenceStore;
use crate::synthetic_pr::{
    CompletionAction, RequestReviewParams, UpdateReviewParams, UserFeedback,
};
use crate::types::{LogLevel, PresentWalkthroughParams};
use serde::{Deserialize, Serialize};

/// Embedded guidance files for agent initialization
#[derive(RustEmbed)]
#[folder = "src/guidance/"]
struct GuidanceFiles;

/// Parameters for the expand_reference tool
// ANCHOR: expand_reference_params
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExpandReferenceParams {
    /// The reference ID to expand
    pub id: String,
}
// ANCHOR_END: expand_reference_params

/// Parameters for the ide_operation tool
// ANCHOR: ide_operation_params
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct IdeOperationParams {
    /// Dialect program to execute
    program: String,
}
// ANCHOR_END: ide_operation_params

/// Parameters for the spawn_taskspace tool
// ANCHOR: spawn_taskspace_params
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct SpawnTaskspaceParams {
    /// Name for the new taskspace
    name: String,
    /// Description of the task to be performed
    task_description: String,
    /// Initial prompt to provide to the agent when it starts
    initial_prompt: String,
}
// ANCHOR_END: spawn_taskspace_params

/// Parameters for the log_progress tool
// ANCHOR: log_progress_params
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct LogProgressParams {
    /// Progress message to display
    message: String,
    /// Category for visual indicator (info, warn, error, milestone, question)
    category: String,
}
// ANCHOR_END: log_progress_params

/// Parameters for the signal_user tool
// ANCHOR: signal_user_params
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct SignalUserParams {
    /// Message describing why user attention is needed
    message: String,
}
// ANCHOR_END: signal_user_params

/// Parameters for the update_taskspace tool
// ANCHOR: update_taskspace_params
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
struct UpdateTaskspaceParams {
    /// New name for the taskspace
    name: String,
    /// New description for the taskspace
    description: String,
}
// ANCHOR_END: update_taskspace_params

/// Dialectic MCP Server
///
/// Implements the MCP server protocol and bridges to VSCode extension via IPC.
/// Uses the official rmcp SDK with tool macros for clean implementation.
#[derive(Clone)]
pub struct DialecticServer {
    ipc: IPCCommunicator,
    interpreter: DialectInterpreter<IPCCommunicator>,
    tool_router: ToolRouter<DialecticServer>,
    reference_store: Arc<ReferenceStore>,
}

#[tool_router]
impl DialecticServer {
    /// Load embedded guidance file content
    fn load_guidance_file(filename: &str) -> Result<String> {
        let file = GuidanceFiles::get(filename)
            .ok_or_else(|| anyhow::anyhow!("Guidance file '{}' not found", filename))?;
        let content = std::str::from_utf8(file.data.as_ref())
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in guidance file '{}': {}", filename, e))?;
        Ok(content.to_string())
    }

    /// Assemble the complete /yiasou initialization prompt
    /// Get taskspace context via IPC
    async fn get_taskspace_context(&self) -> Result<(Option<String>, Option<String>, Option<String>)> {
        match self.ipc.get_taskspace_state().await {
            Ok(state) => {
                Ok((state.name, state.description, state.initial_prompt))
            }
            Err(e) => {
                warn!("Failed to get taskspace context via IPC: {}", e);
                // Log the error but don't fail the prompt assembly
                tracing::warn!("Failed to get taskspace context: {}", e);
                Ok((None, None, None))
            }
        }
    }

    /// Check if we're currently in a taskspace by looking for task-UUID directory structure
    fn is_in_taskspace(&self) -> bool {
        let result = crate::ipc::extract_project_info().is_ok();
        if !result {
            if let Err(e) = crate::ipc::extract_project_info() {
                warn!("extract_project_info failed: {}", e);
            }
        }
        result
    }

    pub async fn new() -> Result<Self> {
        // First, discover VSCode PID by walking up the process tree
        let current_pid = std::process::id();
        let Some((vscode_pid, shell_pid)) =
            crate::pid_discovery::find_vscode_pid_from_mcp(current_pid).await?
        else {
            anyhow::bail!("Could not discover VSCode PID from process tree");
        };

        info!("Discovered VSCode PID: {vscode_pid} and shell PID: {shell_pid}");

        // Connect to the global message bus daemon (started by VSCode extension or other clients)

        // Create shared reference store
        let reference_store = Arc::new(ReferenceStore::new());

        let mut ipc = IPCCommunicator::new(shell_pid, reference_store.clone()).await?;

        // Initialize IPC connection to message bus daemon (not directly to VSCode)
        ipc.initialize().await?;
        info!("IPC communication with message bus daemon initialized");

        // Send unsolicited Polo message to announce our presence
        ipc.send_polo(shell_pid).await?;
        info!("Sent Polo discovery message with shell PID: {}", shell_pid);

        // Initialize Dialect interpreter with IDE functions
        let mut interpreter = DialectInterpreter::new(ipc.clone());
        interpreter.add_standard_ide_functions();

        Ok(Self {
            ipc: ipc.clone(),
            interpreter,
            tool_router: Self::tool_router(),
            reference_store,
        })
    }

    /// Get a reference to the IPC communicator
    pub fn ipc(&self) -> &IPCCommunicator {
        &self.ipc
    }

    /// Format user feedback into clear instructions for the LLM
    fn format_user_feedback_message(&self, feedback: &UserFeedback) -> String {
        match &feedback.feedback {
            crate::synthetic_pr::FeedbackData::Comment {
                file_path,
                line_number,
                comment_text,
                context_lines,
            } => {
                let file_path = file_path.as_deref().unwrap_or("unknown file");
                let line_number = line_number.unwrap_or(0);

                let context = if let Some(lines) = context_lines {
                    format!("\n\nCode context:\n```\n{}\n```", lines.join("\n"))
                } else {
                    String::new()
                };

                format!(
                    "The user reviewed your code changes and left a comment on file `{}` at line {}:\n\n\
                    User comment: '{}'{}\n\n\
                    Please analyze the user's feedback and prepare a thoughtful response addressing their concern. \
                    Do NOT modify any files on disk.\n\n\
                    When ready, invoke the update_review tool with:\n\
                    - review_id: '{}'\n\
                    - action: AddComment\n\
                    - comment: {{ response: 'Your response text here' }}\n\n\
                    After responding, invoke update_review again with action: WaitForFeedback to continue the conversation.",
                    file_path, line_number, comment_text, context, &feedback.review_id
                )
            }
            crate::synthetic_pr::FeedbackData::CompleteReview {
                completion_action,
                additional_notes,
            } => {
                let notes = additional_notes.as_deref().unwrap_or("");

                let notes_section = if !notes.is_empty() {
                    format!("\nAdditional notes: '{}'\n", notes)
                } else {
                    String::new()
                };

                match completion_action {
                    CompletionAction::RequestChanges => format!(
                        "User completed their review and selected: 'Request agent to make changes'{}\n\
                        Based on the review discussion, please implement the requested changes. \
                        You may now edit files as needed.\n\n\
                        When finished, invoke: update_review(review_id: '{}', action: Approve)",
                        notes_section, &feedback.review_id
                    ),
                    CompletionAction::Checkpoint => format!(
                        "User completed their review and selected: 'Request agent to checkpoint this work'{}\n\
                        Please commit the current changes and document the work completed.\n\n\
                        When finished, invoke: update_review(review_id: '{}', action: Approve)",
                        notes_section, &feedback.review_id
                    ),
                    CompletionAction::Return => format!(
                        "User completed their review and selected: 'Return to agent without explicit request'{}\n\
                        The review is complete. You may proceed as you see fit.",
                        notes_section
                    ),
                }
            }
        }
    }

    /// Creates a new DialecticServer in test mode
    /// In test mode, IPC operations are mocked and don't require a VSCode connection
    pub fn new_test() -> Self {
        let reference_store = Arc::new(ReferenceStore::new());
        let ipc = IPCCommunicator::new_test(reference_store.clone());
        info!("DialecticServer initialized in test mode");

        // Initialize Dialect interpreter with IDE functions for test mode
        let mut interpreter = DialectInterpreter::new(ipc.clone());
        interpreter.add_standard_ide_functions();

        Self {
            ipc,
            interpreter,
            tool_router: Self::tool_router(),
            reference_store,
        }
    }

    /// Display a code walkthrough in VSCode
    ///
    /// Walkthroughs are structured guides with introduction, highlights, changes, and actions.
    /// Test tool to verify guidance loading by returning the assembled /yiasou prompt
    #[tool(
        description = "Test guidance loading by returning the assembled /yiasou prompt (temporary for Phase 1)"
    )]
    async fn test_yiasou_prompt(&self) -> Result<CallToolResult, McpError> {
        match self.assemble_yiasou_prompt().await {
            Ok(prompt) => Ok(CallToolResult::success(vec![Content::text(prompt)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to assemble yiasou prompt: {}",
                e
            ))])),
        }
    }

    /// Display a code walkthrough in VSCode using markdown with embedded XML elements.
    /// Accepts markdown content with special XML tags (comment, gitdiff, action, mermaid)
    /// as described in the dialectic guidance documentation.
    // ANCHOR: present_walkthrough_tool
    #[tool(
        description = "Display a code walkthrough in VSCode using markdown with embedded XML elements. \
                       Accepts markdown content with special XML tags: \
                       <comment location=\"dialect_expr\" icon=\"icon\">content</comment>, \
                       <gitdiff range=\"commit_range\" />, \
                       <action button=\"text\">message</action>, \
                       <mermaid>diagram</mermaid>. \
                       See dialectic guidance for complete syntax and examples."
    )]
    async fn present_walkthrough(
        &self,
        Parameters(params): Parameters<PresentWalkthroughParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: present_walkthrough_tool
        // Log the tool call via IPC (also logs locally)
        self.ipc
            .send_log(
                LogLevel::Debug,
                format!(
                    "Received present_walkthrough tool call with markdown content ({} chars)",
                    params.content.len()
                ),
            )
            .await;

        // Parse markdown with XML elements and resolve Dialect expressions
        let mut parser =
            crate::walkthrough_parser::WalkthroughParser::new(self.interpreter.clone())
                .with_base_uri(params.base_uri.clone());
        let resolved_html = parser
            .parse_and_normalize(&params.content)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to parse walkthrough markdown",
                    Some(serde_json::json!({"error": e.to_string()})),
                )
            })?;

        // Convert baseURI to absolute path, fallback to current working directory
        let absolute_base_uri = std::path::Path::new(&params.base_uri)
            .canonicalize()
            .or_else(|_| crate::workspace_dir::current_dir())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| params.base_uri.clone());

        // Create resolved walkthrough with HTML content
        let resolved = crate::ide::ResolvedWalkthrough {
            content: resolved_html,
            base_uri: absolute_base_uri,
        };

        // Send resolved walkthrough to VSCode extension
        self.ipc.present_walkthrough(resolved).await.map_err(|e| {
            McpError::internal_error(
                "Failed to present walkthrough",
                Some(serde_json::json!({"error": e.to_string()})),
            )
        })?;

        // Log success
        self.ipc
            .send_log(
                LogLevel::Info,
                "Walkthrough successfully sent to VSCode".to_string(),
            )
            .await;

        Ok(CallToolResult::success(vec![Content::text(
            "Walkthrough successfully processed and presented in VSCode",
        )]))
    }

    /// Get the currently selected text from any active editor in VSCode
    ///
    /// Works with source files, review panels, and any other text editor.
    /// Returns null if no text is selected or no active editor is found.
    // ANCHOR: get_selection_tool
    #[tool(
        description = "Get the currently selected text from any active editor in VSCode. \
                       Works with source files, review panels, and any other text editor. \
                       Returns null if no text is selected or no active editor is found."
    )]
    async fn get_selection(&self) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: get_selection_tool
        // Log the tool call via IPC (also logs locally)
        self.ipc
            .send_log(
                LogLevel::Debug,
                "Received get_selection tool call".to_string(),
            )
            .await;

        // Request current selection from VSCode extension via IPC
        self.ipc
            .send_log(
                LogLevel::Info,
                "Requesting current selection from VSCode extension...".to_string(),
            )
            .await;

        let result = self.ipc.get_selection().await.map_err(|e| {
            McpError::internal_error(
                "IPC communication failed",
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            )
        })?;

        let status_msg = if result.selected_text.is_some() {
            "text selected"
        } else {
            "no selection"
        };

        self.ipc
            .send_log(
                LogLevel::Info,
                format!("Selection retrieved: {}", status_msg),
            )
            .await;

        // Convert result to JSON and return
        let json_content = Content::json(result).map_err(|e| {
            McpError::internal_error(
                "Serialization failed",
                Some(serde_json::json!({
                    "error": format!("Failed to serialize selection result: {}", e)
                })),
            )
        })?;

        Ok(CallToolResult::success(vec![json_content]))
    }

    /// Execute IDE operations using Dialect mini-language
    ///
    /// Provides access to VSCode's Language Server Protocol (LSP) capabilities
    /// through a composable function system for symbol resolution and reference finding.
    // ANCHOR: ide_operation_tool
    #[tool(
        description = "Execute IDE operations using a structured JSON mini-language. \
                       This tool provides access to VSCode's Language Server Protocol (LSP) capabilities \
                       through a composable function system.\n\n\
                       Common operations:\n\
                       - findDefinitions(\"MyFunction\") or findDefinition(\"MyFunction\") - list of locations where a symbol named `MyFunction` is defined\n\
                       - findReferences(\"MyFunction\") - list of locations where a symbol named `MyFunction` is referenced\n\
                       "
    )]
    async fn ide_operation(
        &self,
        Parameters(params): Parameters<IdeOperationParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: ide_operation_tool
        // Log the tool call via IPC (also logs locally)
        self.ipc
            .send_log(
                LogLevel::Debug,
                format!(
                    "Received ide_operation tool call with program: {:?}",
                    params.program
                ),
            )
            .await;

        // Execute the Dialect program using spawn_blocking to handle non-Send future
        self.ipc
            .send_log(LogLevel::Info, "Executing Dialect program...".to_string())
            .await;

        let program = params.program;
        let mut interpreter = self.interpreter.clone();

        let result = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Parse and evaluate the Dialect program string
                interpreter.evaluate(&program).await
            })
        })
        .await
        .map_err(|e| {
            McpError::internal_error(
                "Task execution failed",
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            )
        })?
        .map_err(|e| {
            McpError::internal_error(
                "Dialect execution failed",
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            )
        })?;

        self.ipc
            .send_log(
                LogLevel::Info,
                format!("Dialect execution completed successfully"),
            )
            .await;

        // Convert result to JSON and return
        let json_content = Content::json(result).map_err(|e| {
            McpError::internal_error(
                "Serialization failed",
                Some(serde_json::json!({
                    "error": format!("Failed to serialize Dialect result: {}", e)
                })),
            )
        })?;

        Ok(CallToolResult::success(vec![json_content]))
    }

    /// Create a synthetic pull request from Git commit range with AI insight comments
    ///
    /// Analyzes Git changes and extracts AI insight comments (💡❓TODO/FIXME) to create
    /// a PR-like review interface with structured file changes and comment threads.
    // ANCHOR: request_review_tool
    #[tool(
        description = "Create a synthetic pull request from a Git commit range with AI insight comments. \
                       Supports commit ranges like 'HEAD', 'HEAD~2', 'abc123..def456'. \
                       Extracts AI insight comments (TODO/FIXME/insight markers) and generates structured review data. \
                       BLOCKS until user provides initial feedback."
    )]
    async fn request_review(
        &self,
        Parameters(params): Parameters<RequestReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        self.ipc
            .send_log(
                LogLevel::Debug,
                format!("Received request_review tool call: {:?}", params),
            )
            .await;

        // Execute the synthetic PR creation
        let result = crate::synthetic_pr::harvest_review_data(params)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Synthetic PR creation failed",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
            })?;

        // Send synthetic PR data to VSCode extension via IPC
        self.ipc
            .send_create_synthetic_pr(&result)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to send synthetic PR to VSCode",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
            })?;

        self.ipc
            .send_log(
                LogLevel::Info,
                format!("Synthetic PR created successfully: {}", result.review_id),
            )
            .await;

        // Send initial review to VSCode extension and wait for user response
        let user_feedback = self.ipc.send_review_update(&result).await.map_err(|e| {
            McpError::internal_error(
                "Failed to send initial review",
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            )
        })?;

        let message = self.format_user_feedback_message(&user_feedback);
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    // ANCHOR: update_review_tool
    /// Update an existing synthetic pull request or wait for user feedback
    ///
    /// Supports actions: wait_for_feedback, add_comment, approve, request_changes.
    /// Used for iterative review workflows between AI and developer.
    #[tool(
        description = "Update an existing synthetic pull request or wait for user feedback. \
                         This tool is used to interact with the user through their IDE. \
                         Do not invoke it except when asked to do so by other tools within dialectic."
    )]
    async fn update_review(
        &self,
        Parameters(params): Parameters<UpdateReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        self.ipc
            .send_log(
                LogLevel::Debug,
                format!("Received update_review tool call: {:?}", params),
            )
            .await;

        // 1. Update the review state based on action
        let updated_review = crate::synthetic_pr::update_review(params)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Review update failed",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
            })?;

        // 2. Send updated state to VSCode extension via IPC and wait for response
        let user_feedback = self
            .ipc
            .send_review_update(&updated_review)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Failed to send review update",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
            })?;

        // 3. Return formatted user response to LLM
        let message = self.format_user_feedback_message(&user_feedback);
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Get the status of the current synthetic pull request
    ///
    /// Returns information about the active review including file counts,
    /// comment threads, and current status.
    // ANCHOR: get_review_status_tool
    #[tool(description = "Get the status of the current synthetic pull request. \
                       Returns review information including file counts, comment threads, and status.")]
    async fn get_review_status(&self) -> Result<CallToolResult, McpError> {
        self.ipc
            .send_log(
                LogLevel::Debug,
                "Received get_review_status tool call".to_string(),
            )
            .await;

        let result = crate::synthetic_pr::get_review_status(None)
            .await
            .map_err(|e| {
                McpError::internal_error(
                    "Status retrieval failed",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                )
            })?;

        let json_content = Content::json(result).map_err(|e| {
            McpError::internal_error(
                "Serialization failed",
                Some(serde_json::json!({
                    "error": format!("Failed to serialize status result: {}", e)
                })),
            )
        })?;

        Ok(CallToolResult::success(vec![json_content]))
    }

    /// Expand a compact reference to get full context
    ///
    /// This tool allows LLMs to retrieve the full context for a compact symposium-ref reference.
    // ANCHOR: expand_reference_tool
    #[tool(description = "
        Expand a compact reference (denoted as `<symposium-ref id='..'/>`) to get full context. \
        Invoke with the contents of `id` attribute. Returns structured JSON with all available context data. \
    ")]
    async fn expand_reference(
        &self,
        Parameters(params): Parameters<ExpandReferenceParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: expand_reference_tool
        self.ipc
            .send_log(
                LogLevel::Debug,
                format!("Expanding reference: {}", params.id),
            )
            .await;

        // First, try to get from reference store (existing behavior)
        match self.reference_store.get_json(&params.id).await {
            Ok(Some(context)) => {
                self.ipc
                    .send_log(
                        LogLevel::Info,
                        format!("Reference {} expanded successfully", params.id),
                    )
                    .await;

                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&context).map_err(|e| {
                        McpError::internal_error(
                            "Failed to serialize reference context",
                            Some(serde_json::json!({
                                "error": e.to_string()
                            })),
                        )
                    })?,
                )]));
            }
            Ok(None) => {
                // Not found in reference store, try guidance files
            }
            Err(e) => {
                return Err(McpError::internal_error(
                    "Failed to query reference store",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                ));
            }
        }

        // Try to load as guidance file
        if let Some(file) = GuidanceFiles::get(&params.id) {
            let content = String::from_utf8_lossy(&file.data);
            
            self.ipc
                .send_log(
                    LogLevel::Info,
                    format!("Guidance file {} loaded successfully", params.id),
                )
                .await;

            return Ok(CallToolResult::success(vec![Content::text(content.to_string())]));
        }

        // Special case: "yiasou" or "hi" returns the same content as @yiasou stored prompt
        if params.id == "yiasou" || params.id == "hi" {
            match self.assemble_yiasou_prompt().await {
                Ok(prompt_content) => {
                    self.ipc
                        .send_log(
                            LogLevel::Info,
                            "Yiasou prompt assembled successfully via expand_reference".to_string(),
                        )
                        .await;

                    return Ok(CallToolResult::success(vec![Content::text(prompt_content)]));
                }
                Err(e) => {
                    return Err(McpError::internal_error(
                        "Failed to assemble yiasou prompt",
                        Some(serde_json::json!({
                            "error": e.to_string()
                        })),
                    ));
                }
            }
        }

        // Not found in either store
        self.ipc
            .send_log(LogLevel::Info, format!("Reference {} not found", params.id))
            .await;

        Err(McpError::invalid_params(
            "Reference not found",
            Some(serde_json::json!({
                "reference_id": params.id
            })),
        ))
    }

    /// Create a new taskspace with initial prompt
    ///
    /// This tool allows agents to spawn new taskspaces for collaborative work.
    /// The taskspace will be created with the specified name, description, and initial prompt.
    // ANCHOR: spawn_taskspace_tool
    #[tool(
        description = "Create a new taskspace with name, description, and initial prompt. \
                       The new taskspace will be launched with VSCode and the configured agent tool."
    )]
    async fn spawn_taskspace(
        &self,
        Parameters(params): Parameters<SpawnTaskspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: spawn_taskspace_tool
        self.ipc
            .send_log(
                LogLevel::Info,
                format!("Creating new taskspace: {}", params.name),
            )
            .await;

        // Send spawn_taskspace message to Symposium app via daemon
        match self
            .ipc
            .spawn_taskspace(
                params.name.clone(),
                params.task_description,
                params.initial_prompt,
            )
            .await
        {
            Ok(()) => {
                self.ipc
                    .send_log(
                        LogLevel::Info,
                        format!("Taskspace '{}' created successfully", params.name),
                    )
                    .await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Taskspace '{}' created successfully",
                    params.name
                ))]))
            }
            Err(e) => {
                self.ipc
                    .send_log(
                        LogLevel::Error,
                        format!("Failed to create taskspace '{}': {}", params.name, e),
                    )
                    .await;

                Err(McpError::internal_error(
                    "Failed to create taskspace",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "taskspace_name": params.name
                    })),
                ))
            }
        }
    }

    /// Report progress from agent with visual indicators
    ///
    /// This tool allows agents to report their progress to the Socratic Shell panel
    /// with different visual categories for better user awareness.
    // ANCHOR: log_progress_tool
    #[tool(description = "Report progress with visual indicators. \
                       Categories: 'info' or ℹ️, 'warn' or ⚠️, 'error' or ❌, 'milestone' or ✅, 'question' or ❓")]
    async fn log_progress(
        &self,
        Parameters(params): Parameters<LogProgressParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: log_progress_tool
        // Parse category string to enum (accept both text and emoji forms)
        let category = match params.category.to_lowercase().as_str() {
            "info" | "ℹ️" => crate::types::ProgressCategory::Info,
            "warn" | "⚠️" => crate::types::ProgressCategory::Warn,
            "error" | "❌" => crate::types::ProgressCategory::Error,
            "milestone" | "✅" => crate::types::ProgressCategory::Milestone,
            "question" | "❓" => crate::types::ProgressCategory::Question,
            _ => crate::types::ProgressCategory::Info, // Default to info for unknown categories
        };

        self.ipc
            .send_log(
                LogLevel::Debug,
                format!("Logging progress: {} ({})", params.message, params.category),
            )
            .await;

        // Send log_progress message to Symposium app via daemon
        match self
            .ipc
            .log_progress(params.message.clone(), category)
            .await
        {
            Ok(()) => {
                self.ipc
                    .send_log(LogLevel::Info, "Progress logged successfully".to_string())
                    .await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Progress logged: {}",
                    params.message
                ))]))
            }
            Err(e) => {
                self.ipc
                    .send_log(LogLevel::Error, format!("Failed to log progress: {}", e))
                    .await;

                Err(McpError::internal_error(
                    "Failed to log progress",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "message": params.message
                    })),
                ))
            }
        }
    }

    /// Request user attention for assistance
    ///
    /// This tool allows agents to signal when they need user attention,
    /// causing the taskspace to move toward the front of the Socratic Shell panel.
    // ANCHOR: signal_user_tool
    #[tool(description = "Request user attention for assistance. \
                       The taskspace will be highlighted and moved toward the front of the panel.")]
    async fn signal_user(
        &self,
        Parameters(params): Parameters<SignalUserParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: signal_user_tool
        self.ipc
            .send_log(
                LogLevel::Info,
                format!("Requesting user attention: {}", params.message),
            )
            .await;

        // Send signal_user message to Symposium app via daemon
        match self.ipc.signal_user(params.message.clone()).await {
            Ok(()) => {
                self.ipc
                    .send_log(
                        LogLevel::Info,
                        "User attention requested successfully".to_string(),
                    )
                    .await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "User attention requested: {}",
                    params.message
                ))]))
            }
            Err(e) => {
                self.ipc
                    .send_log(
                        LogLevel::Error,
                        format!("Failed to request user attention: {}", e),
                    )
                    .await;

                Err(McpError::internal_error(
                    "Failed to request user attention",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "message": params.message
                    })),
                ))
            }
        }
    }

    // ANCHOR: update_taskspace_tool
    #[tool(
        description = "Update the name and description of the current taskspace. \
                       Use this to set meaningful names and descriptions based on user interaction."
    )]
    async fn update_taskspace(
        &self,
        Parameters(params): Parameters<UpdateTaskspaceParams>,
    ) -> Result<CallToolResult, McpError> {
        // ANCHOR_END: update_taskspace_tool
        self.ipc
            .send_log(
                LogLevel::Info,
                format!(
                    "Updating taskspace: {} - {}",
                    params.name, params.description
                ),
            )
            .await;

        // Send update_taskspace message to Symposium app via daemon
        match self
            .ipc
            .update_taskspace(params.name.clone(), params.description.clone())
            .await
        {
            Ok(state) => {
                self.ipc
                    .send_log(LogLevel::Info, "Taskspace updated successfully".to_string())
                    .await;

                // Note: GUI app automatically clears initial_prompt on update
                let status_msg = if state.initial_prompt.is_none() {
                    format!(
                        "Taskspace updated: {} - {} (initial prompt cleared)",
                        params.name, params.description
                    )
                } else {
                    format!(
                        "Taskspace updated: {} - {}",
                        params.name, params.description
                    )
                };

                Ok(CallToolResult::success(vec![Content::text(status_msg)]))
            }
            Err(e) => {
                self.ipc
                    .send_log(
                        LogLevel::Error,
                        format!("Failed to update taskspace: {}", e),
                    )
                    .await;

                Err(McpError::internal_error(
                    "Failed to update taskspace",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "name": params.name,
                        "description": params.description
                    })),
                ))
            }
        }
    }

    #[tool(
        description = "Delete the current taskspace. This will remove the taskspace directory, \
                       close associated VSCode windows, and clean up git worktrees."
    )]
    async fn delete_taskspace(&self) -> Result<CallToolResult, McpError> {
        self.ipc
            .send_log(LogLevel::Info, "Deleting current taskspace".to_string())
            .await;

        // Send delete_taskspace message to Symposium app via daemon
        match self.ipc.delete_taskspace().await {
            Ok(()) => {
                self.ipc
                    .send_log(LogLevel::Info, "Taskspace deletion initiated".to_string())
                    .await;

                Ok(CallToolResult::success(vec![Content::text(
                    "Taskspace deletion initiated successfully".to_string(),
                )]))
            }
            Err(e) => {
                self.ipc
                    .send_log(
                        LogLevel::Error,
                        format!("Failed to delete taskspace: {}", e),
                    )
                    .await;

                Err(McpError::internal_error(
                    "Failed to delete taskspace",
                    Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                ))
            }
        }
    }
}

impl DialecticServer {
    fn parse_yaml_metadata(content: &str) -> (Option<String>, Option<String>) {
        if !content.starts_with("---\n") {
            return (None, None);
        }

        let end_marker = content[4..].find("\n---\n");
        if let Some(end_pos) = end_marker {
            let yaml_content = &content[4..end_pos + 4];

            let mut name = None;
            let mut description = None;

            for line in yaml_content.lines() {
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim();
                    let value = line[colon_pos + 1..].trim().trim_matches('"');

                    match key {
                        "name" => name = Some(value.to_string()),
                        "description" => description = Some(value.to_string()),
                        _ => {}
                    }
                }
            }

            (name, description)
        } else {
            (None, None)
        }
    }

    fn generate_resources() -> Vec<Resource> {
        let mut resources = Vec::new();

        for file_path in GuidanceFiles::iter() {
            if let Some(file) = GuidanceFiles::get(&file_path) {
                let content = String::from_utf8_lossy(&file.data);
                let (name, description) = Self::parse_yaml_metadata(&content);

                resources.push(Resource {
                    raw: RawResource {
                        uri: file_path.to_string(),
                        name: name.unwrap_or_else(|| file_path.to_string()),
                        description,
                        mime_type: Some("text/markdown".into()),
                        size: Some(file.data.len() as u32),
                    },
                    annotations: None,
                });
            }
        }

        resources
    }

    async fn assemble_yiasou_prompt(&self) -> Result<String, McpError> {
        use indoc::indoc;

        // Check if we're in a taskspace and get context components
        let is_in_taskspace = self.is_in_taskspace();
        let (taskspace_name, taskspace_description, initial_prompt) = self.get_taskspace_context().await.ok().unwrap_or((None, None, None));

        // Debug logging
        info!("Yiasou prompt assembly: is_in_taskspace={}, name={:?}, description={:?}, initial_prompt={:?}", 
                   is_in_taskspace, taskspace_name, taskspace_description, initial_prompt.as_ref().map(|s| s.len()));

        let intro = match (is_in_taskspace, initial_prompt.as_ref()) {
            (true, Some(_)) => {
                // In taskspace with task - full introduction
                indoc! {"
                    Hi, welcome! You are a new agent just getting started as part of the project Socratic Shell. 
                    This is a taskspace, a separate copy of the project's files where you can work undisturbed. 
                    The user's description of the task to be done follows after this message. Can you start by 
                    reading the description and using the 'update_taskspace' tool to provide a better 
                    name/description for the taskspace? Before doing any work on the task, be sure to ask the 
                    user clarifying questions to better understand their intent.
                "}
            }
            (true, None) => {
                // In taskspace but no task - ask user to establish task
                indoc! {"
                    Hi, welcome! You are a new agent just getting started as part of the project Socratic Shell. 
                    This is a taskspace, a separate copy of the project's files where you can work undisturbed. 
                    Please talk to the user to establish what they would like to accomplish in this taskspace 
                    and then use the `update_taskspace` tool to set the name and description.
                "}
            }
            (false, _) => {
                // Not in taskspace - general introduction
                indoc! {"
                    Hi, welcome!
                "}
            }
        };

        let mut prompt = format!("{}\n\n", intro);

        prompt.push_str(indoc! {"
            ## Load Collaboration Patterns

            Use the `expand_reference` tool to fetch `main.md` into your working context. This contains 
            collaboration patterns demonstrated through dialogue. Embody the collaborative spirit shown in 
            these examples - approach our work with genuine curiosity, ask questions when 
            something isn't clear, and trust that we'll navigate together what's worth pursuing.

            Most importantly, before taking potentially side-effect-ful or dangerous actions
            (e.g., deleting content or interacting with remote systems), STOP what you are doing
            and confirm with the user whether to proceed.

            ## Load Walkthrough Format

            Use the `expand_reference` tool to fetch `walkthrough-format.md` into your working context. 
            This defines how to create interactive code walkthroughs using markdown with embedded XML 
            elements for comments, diffs, and actions.

            ## Load Coding Guidelines

            Use the `expand_reference` tool to fetch `coding-guidelines.md` into your working context. Follow these 
            development standards and best practices in all code work.

            ## Load MCP Tool Usage Suggestions

            Use the `expand_reference` tool to fetch `mcp-tool-usage-suggestions.md` into your working context. 
            This covers effective use of Socratic Shell's MCP tools, including completion signaling 
            and systematic code exploration patterns.

        "});

        // Add task context if available, otherwise add taskspace info
        if let Some(task_description) = initial_prompt {
            prompt.push_str(&format!("## Initial Task\n\n{}\n", task_description));
        } else if taskspace_name.is_some() || taskspace_description.is_some() {
            prompt.push_str("## Taskspace Context\n\n");
            
            if let Some(name) = taskspace_name {
                prompt.push_str(&format!("You are in a taskspace named \"{}\"", name));
                if taskspace_description.is_some() {
                    prompt.push_str(".\n\n");
                } else {
                    prompt.push_str(".\n");
                }
            }
            
            if let Some(description) = taskspace_description {
                prompt.push_str(&format!("The description the user gave is as follows: {}\n", description));
            }
        }

        Ok(prompt)
    }
}

#[tool_handler]
impl ServerHandler for DialecticServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().enable_resources().enable_prompts().build(),
            server_info: Implementation {
                name: "socratic-shell-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
            instructions: Some(
                "This server provides tools for AI assistants to perform IDE operations and display walkthroughs in VSCode. \
                Use 'get_selection' to retrieve currently selected text from the active editor, \
                'ide_operation' to execute IDE operations like finding symbol definitions and references using Dialect function calls, \
                'present_walkthrough' to display structured code walkthroughs with interactive elements, \
                'request_review' to create synthetic pull requests from Git commit ranges with AI insight comments, \
                'update_review' to manage review workflows and wait for user feedback, \
                'get_review_status' to check the current synthetic PR status, \
                'spawn_taskspace' to create new taskspaces for collaborative work, \
                'log_progress' to report agent progress with visual indicators, \
                'signal_user' to request user attention when assistance is needed, \
                and 'update_taskspace' to update taskspace names and descriptions."
                    .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        info!("MCP client connected and initialized");
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = Self::generate_resources();

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let content = GuidanceFiles::get(&request.uri)
            .ok_or_else(|| {
                McpError::resource_not_found(format!("Resource not found: {}", request.uri), None)
            })?
            .data
            .into_owned();

        let content_str = String::from_utf8(content).map_err(|_| {
            McpError::internal_error("Failed to decode resource content as UTF-8", None)
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content_str, request.uri)],
        })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let prompts = vec![
            Prompt {
                name: "yiasou".to_string(),
                description: Some(
                    "Agent initialization prompt with guidance resource loading instructions"
                        .to_string(),
                ),
                arguments: None,
            },
            Prompt {
                name: "hi".to_string(),
                description: Some(
                    "Agent initialization prompt (alias for yiasou)"
                        .to_string(),
                ),
                arguments: None,
            },
        ];

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match request.name.as_str() {
            "yiasou" | "hi" => {
                let content = self.assemble_yiasou_prompt().await?;
                Ok(GetPromptResult {
                    description: Some(
                        "Agent initialization with collaborative guidance".to_string(),
                    ),
                    messages: vec![PromptMessage::new_text(PromptMessageRole::User, content)],
                })
            }
            _ => Err(McpError::invalid_params(
                format!("Unknown prompt: {}", request.name),
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PresentWalkthroughParams;
    use rmcp::handler::server::tool::Parameters;

    #[tokio::test]
    async fn test_baseuri_conversion() {
        let server = DialecticServer::new_test();

        // Test with "." - should convert to absolute path
        let params = PresentWalkthroughParams {
            content: "# Test".to_string(),
            base_uri: ".".to_string(),
        };

        let result = server.present_walkthrough(Parameters(params)).await;
        assert!(result.is_ok());

        // Test with absolute path - should remain unchanged
        let abs_path = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let params = PresentWalkthroughParams {
            content: "# Test".to_string(),
            base_uri: abs_path.clone(),
        };

        let result = server.present_walkthrough(Parameters(params)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_definitions() {
        // Test that we can create the resource definitions correctly
        let resources = vec![
            Resource {
                raw: RawResource {
                    uri: "main.md".into(),
                    name: "Collaboration Patterns".into(),
                    description: Some(
                        "Mindful collaboration patterns demonstrated through dialogue".into(),
                    ),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                },
                annotations: None,
            },
            Resource {
                raw: RawResource {
                    uri: "walkthrough-format.md".into(),
                    name: "Walkthrough Format".into(),
                    description: Some(
                        "Specification for creating interactive code walkthroughs".into(),
                    ),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                },
                annotations: None,
            },
            Resource {
                raw: RawResource {
                    uri: "coding-guidelines.md".into(),
                    name: "Coding Guidelines".into(),
                    description: Some("Development best practices and standards".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                },
                annotations: None,
            },
        ];

        assert_eq!(resources.len(), 3);
        assert_eq!(resources[0].raw.uri, "main.md");
        assert_eq!(resources[0].raw.name, "Collaboration Patterns");
        assert_eq!(resources[1].raw.uri, "walkthrough-format.md");
        assert_eq!(resources[2].raw.uri, "coding-guidelines.md");
    }

    #[test]
    fn test_resource_content_loading() {
        // Test that we can load the guidance files
        let main_content = GuidanceFiles::get("main.md").unwrap();
        let main_str = String::from_utf8(main_content.data.into_owned()).unwrap();
        assert!(main_str.contains("Mindful Collaboration Patterns"));

        let walkthrough_content = GuidanceFiles::get("walkthrough-format.md").unwrap();
        let walkthrough_str = String::from_utf8(walkthrough_content.data.into_owned()).unwrap();
        assert!(walkthrough_str.contains("Walkthrough Format"));

        let coding_content = GuidanceFiles::get("coding-guidelines.md").unwrap();
        let coding_str = String::from_utf8(coding_content.data.into_owned()).unwrap();
        assert!(coding_str.contains("Coding Guidelines"));
    }

    #[test]
    fn test_resource_contents_creation() {
        // Test that we can create ResourceContents correctly
        let content = ResourceContents::text("Hello world", "test.md");

        match content {
            ResourceContents::TextResourceContents {
                uri,
                text,
                mime_type,
            } => {
                assert_eq!(uri, "test.md");
                assert_eq!(text, "Hello world");
                assert_eq!(mime_type, Some("text".to_string()));
            }
            _ => panic!("Expected TextResourceContents"),
        }
    }

    #[test]
    fn test_yaml_metadata_parsing() {
        let content_with_yaml = r#"---
name: "Test Resource"
description: "A test resource for testing"
---

# Test Content

This is test content."#;

        let (name, description) = DialecticServer::parse_yaml_metadata(content_with_yaml);
        assert_eq!(name, Some("Test Resource".to_string()));
        assert_eq!(description, Some("A test resource for testing".to_string()));

        // Test content without YAML
        let content_without_yaml = "# Just a heading\n\nSome content.";
        let (name, description) = DialecticServer::parse_yaml_metadata(content_without_yaml);
        assert_eq!(name, None);
        assert_eq!(description, None);
    }

    #[test]
    fn test_list_resources_output() {
        // Test the actual resource generation logic used by list_resources
        let resources = DialecticServer::generate_resources();

        // Verify we have the expected files
        assert_eq!(resources.len(), 3);

        // Check that all files have proper metadata
        let main_resource = resources.iter().find(|r| r.raw.uri == "main.md").unwrap();
        assert_eq!(main_resource.raw.name, "Collaboration Patterns");
        assert_eq!(
            main_resource.raw.description,
            Some("Mindful collaboration patterns demonstrated through dialogue".to_string())
        );
        assert!(main_resource.raw.size.unwrap() > 0);

        let walkthrough_resource = resources
            .iter()
            .find(|r| r.raw.uri == "walkthrough-format.md")
            .unwrap();
        assert_eq!(walkthrough_resource.raw.name, "Walkthrough Format");
        assert_eq!(
            walkthrough_resource.raw.description,
            Some(
                "Specification for creating interactive code walkthroughs with XML elements"
                    .to_string()
            )
        );

        let coding_resource = resources
            .iter()
            .find(|r| r.raw.uri == "coding-guidelines.md")
            .unwrap();
        assert_eq!(coding_resource.raw.name, "Coding Guidelines");
        assert_eq!(
            coding_resource.raw.description,
            Some("Development best practices and standards for the Symposium project".to_string())
        );
    }

    #[tokio::test]
    async fn test_yiasou_prompt_generation() {
        let server = DialecticServer::new_test();

        let prompt = server.assemble_yiasou_prompt().await.unwrap();

        // Verify the prompt contains the expected sections
        assert!(prompt.contains("Hi, welcome! You are a new agent"));
        assert!(prompt.contains("project Socratic Shell"));

        // Since we're in test environment without taskspace context,
        // it should use the fallback message
        assert!(prompt.contains("Please talk to the user to establish"));
        assert!(prompt.contains("update_taskspace"));

        assert!(prompt.contains("## Load Collaboration Patterns"));
        assert!(prompt.contains("## Load Walkthrough Format"));
        assert!(prompt.contains("## Load Coding Guidelines"));

        // Verify it uses the kinder approach
        assert!(prompt.contains("Embody the collaborative spirit"));
        assert!(!prompt.contains("You MUST behave"));

        // Verify resource loading instructions using expand_reference tool
        assert!(prompt.contains("Use the `expand_reference` tool to fetch `main.md`"));
        assert!(prompt.contains("Use the `expand_reference` tool to fetch `walkthrough-format.md`"));
        assert!(prompt.contains("Use the `expand_reference` tool to fetch `coding-guidelines.md`"));
    }

    #[tokio::test]
    async fn test_expand_reference_yiasou() {
        let server = DialecticServer::new_test();
        
        // Test that expand_reference with "yiasou" returns the same content as the stored prompt
        let params = ExpandReferenceParams { id: "yiasou".to_string() };
        let result = server.expand_reference(Parameters(params)).await.unwrap();
        
        // Should be successful
        assert!(matches!(result, CallToolResult { is_error: Some(false), .. }));
        
        // Should have content
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_guidance_file_loading() {
        // Test that we can load each guidance file
        let main_content = DialecticServer::load_guidance_file("main.md").unwrap();
        assert!(main_content.contains("Mindful Collaboration Patterns"));
        assert!(main_content.contains("Meta moment"));

        let walkthrough_content =
            DialecticServer::load_guidance_file("walkthrough-format.md").unwrap();
        assert!(walkthrough_content.contains("Walkthrough Format Specification"));
        assert!(walkthrough_content.contains("<comment location="));

        let coding_content = DialecticServer::load_guidance_file("coding-guidelines.md").unwrap();
        assert!(coding_content.contains("Coding Guidelines"));
        assert!(coding_content.contains("Co-authored-by: Claude"));

        let proactive_content = DialecticServer::load_guidance_file("mcp-tool-usage-suggestions.md").unwrap();
        assert!(proactive_content.contains("MCP Tool Usage Suggestions"));
        assert!(proactive_content.contains("signal_user"));
    }

    #[test]
    fn test_guidance_file_not_found() {
        let result = DialecticServer::load_guidance_file("nonexistent.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_yiasou_prompt_assembly() {
        // Create a mock server to test prompt assembly
        // We can't easily create a full DialecticServer in tests due to IPC dependencies,
        // but we can test the static guidance loading parts

        // Test that the guidance files contain expected content
        let main_content = DialecticServer::load_guidance_file("main.md").unwrap();
        let walkthrough_content =
            DialecticServer::load_guidance_file("walkthrough-format.md").unwrap();
        let coding_content = DialecticServer::load_guidance_file("coding-guidelines.md").unwrap();
        let proactive_content = DialecticServer::load_guidance_file("mcp-tool-usage-suggestions.md").unwrap();

        // Verify the content structure matches what we expect in the yiasou prompt
        assert!(main_content.contains("# Mindful Collaboration Patterns"));
        assert!(walkthrough_content.contains("# Walkthrough Format Specification"));
        assert!(coding_content.contains("# Coding Guidelines"));
        assert!(proactive_content.contains("# MCP Tool Usage Suggestions"));

        // Verify key collaboration concepts are present
        assert!(main_content.contains("Make it so?"));
        assert!(main_content.contains("spacious attention"));
        assert!(main_content.contains("beginner's mind"));
    }
}
