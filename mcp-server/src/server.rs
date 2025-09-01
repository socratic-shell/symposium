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
use serde_json;
use std::future::Future;
use std::sync::Arc;
use tracing::info;

use crate::dialect::DialectInterpreter;
use crate::ipc::IPCCommunicator;
use crate::reference_store::ReferenceStore;
use crate::synthetic_pr::{
    CompletionAction, RequestReviewParams, UpdateReviewParams, UserFeedback,
};
use crate::types::{LogLevel, PresentWalkthroughParams};
use serde::{Deserialize, Serialize};

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
        interpreter.add_function::<crate::ide::FindDefinitions>();
        interpreter.add_function::<crate::ide::FindReferences>();
        interpreter.add_function::<crate::ide::Search>();
        interpreter.add_function::<crate::ide::Lines>();
        interpreter.add_function::<crate::ide::GitDiff>();
        interpreter.add_function::<crate::ide::Comment>();
        interpreter.add_function::<crate::ide::Action>();

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
        interpreter.add_function::<crate::ide::FindDefinitions>();
        interpreter.add_function::<crate::ide::FindReferences>();
        interpreter.add_function::<crate::ide::Search>();
        interpreter.add_function::<crate::ide::Lines>();
        interpreter.add_function::<crate::ide::GitDiff>();
        interpreter.add_function::<crate::ide::Comment>();
        interpreter.add_function::<crate::ide::Action>();

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
    /// Display a code walkthrough in VSCode using markdown with embedded XML elements.
    /// Accepts markdown content with special XML tags (comment, gitdiff, action, mermaid)
    /// as described in the dialectic guidance documentation.
    // ANCHOR: present_walkthrough_tool
    #[tool(description = "Display a code walkthrough in VSCode using markdown with embedded XML elements. \
                       Accepts markdown content with special XML tags: \
                       <comment location=\"dialect_expr\" icon=\"icon\">content</comment>, \
                       <gitdiff range=\"commit_range\" />, \
                       <action button=\"text\">message</action>, \
                       <mermaid>diagram</mermaid>. \
                       See dialectic guidance for complete syntax and examples.")]
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
        let mut parser = crate::walkthrough_parser::WalkthroughParser::new(self.interpreter.clone());
        let resolved_html = parser.parse_and_normalize(&params.content).await.map_err(|e| {
            McpError::internal_error(
                "Failed to parse walkthrough markdown",
                Some(serde_json::json!({"error": e.to_string()})),
            )
        })?;

        // Create resolved walkthrough with HTML content
        let resolved = crate::ide::ResolvedWalkthrough {
            content: resolved_html,
            base_uri: params.base_uri.clone(),
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
                       - findDefinitions(\"MyFunction\") - list of locations where a symbol named `MyFunction` is defined\n\
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
        Invoke with the contents of `id` attribute. \
        Returns structured JSON with all available context data. \
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

        match self.reference_store.get(&params.id).await {
            Ok(Some(context)) => {
                self.ipc
                    .send_log(
                        LogLevel::Info,
                        format!("Reference {} expanded successfully", params.id),
                    )
                    .await;

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&context).map_err(|e| {
                        McpError::internal_error(
                            "Failed to serialize reference context",
                            Some(serde_json::json!({
                                "error": e.to_string()
                            })),
                        )
                    })?,
                )]))
            }
            Ok(None) => {
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
            Err(e) => {
                self.ipc
                    .send_log(
                        LogLevel::Error,
                        format!("Error expanding reference {}: {}", params.id, e),
                    )
                    .await;

                Err(McpError::internal_error(
                    "Failed to expand reference",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "reference_id": params.id
                    })),
                ))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for DialecticServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "symposium-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
            instructions: Some(
                "This server provides tools for AI assistants to perform IDE operations and display walkthroughs in VSCode. \
                Use 'get_selection' to retrieve currently selected text from the active editor, \
                'ide_operation' to execute IDE operations like finding symbol definitions and references using Dialect function calls, \
                'present_walkthrough' to display structured code walkthroughs with interactive elements, \
                'request_review' to create synthetic pull requests from Git commit ranges with AI insight comments, \
                'update_review' to manage review workflows and wait for user feedback, \
                and 'get_review_status' to check the current synthetic PR status."
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
}
