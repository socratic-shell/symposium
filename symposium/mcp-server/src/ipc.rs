//! IPC communication module for Dialectic MCP Server
//!
//! Handles Unix socket/named pipe communication with the VSCode extension.
//! Ports the logic from server/src/ipc.ts to Rust with cross-platform support.

use crate::{constants::DAEMON_SOCKET_PREFIX, types::{
    FindAllReferencesPayload, GetSelectionMessage, GetSelectionResult, LogLevel, ResolveSymbolByNamePayload
}};
use anyhow::Context;

use serde_json;
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Extract project path and taskspace UUID from current working directory
///
/// Expected directory structure: `project.symposium/task-$UUID/$checkout/`
/// Traverses upward looking for `task-$UUID` directories and stops at `.symposium`.
/// Uses the last UUID found during traversal.
pub fn extract_project_info() -> Result<(String, String)> {
    let current_dir = crate::workspace_dir::current_dir()
        .map_err(|e| IPCError::Other(format!("Failed to get current working directory: {}", e)))?;

    let mut dir = current_dir.as_path();
    let mut last_uuid = None;

    loop {
        // Check if current directory name matches task-$UUID pattern
        if let Some(dir_name) = dir.file_name().and_then(|name| name.to_str()) {
            if let Some(uuid_part) = dir_name.strip_prefix("task-") {
                // Try to parse as UUID
                if let Ok(uuid) = Uuid::parse_str(uuid_part) {
                    last_uuid = Some(uuid.to_string());
                }
            }

            // Check if we've reached a .symposium directory
            if dir_name.ends_with(".symposium") {
                let project_path = dir.to_string_lossy().to_string();
                let taskspace_uuid = last_uuid.ok_or_else(|| {
                    IPCError::Other(
                        "No task-$UUID directory found before reaching .symposium".to_string(),
                    )
                })?;

                return Ok((project_path, taskspace_uuid));
            }
        }

        // Move to parent directory
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    Err(IPCError::Other(
        "No .symposium directory found in directory tree".to_string(),
    ))
}

/// Errors that can occur during IPC communication
#[derive(Error, Debug)]
pub enum IPCError {
    #[error("Environment variable DIALECTIC_IPC_PATH not set")]
    MissingEnvironmentVariable,

    #[error("Failed to connect to socket/pipe at {path}: {source}")]
    ConnectionFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("IPC connection not established")]
    NotConnected,

    #[error("Failed to serialize message: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Failed to write to IPC connection: {0}")]
    WriteError(#[from] std::io::Error),

    #[error("Request timeout after 5 seconds")]
    Timeout,

    #[error("Response channel closed")]
    ChannelClosed,

    #[error("Failed to send message: {0}")]
    SendError(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, IPCError>;

/// Handles IPC communication between MCP server and VSCode extension
///
/// IPC communication using actor-based dispatch system.
/// All messages now use the actor system for clean, testable architecture.
#[derive(Clone)]
pub struct IPCCommunicator {
    /// Actor-based dispatch system for all IPC messages
    dispatch_handle: crate::actor::DispatchHandle,

    /// Terminal shell PID for this MCP server instance
    /// Reported to extension during handshake for smart terminal selection
    /// None when VSCode PID discovery fails (e.g., persistent agents)
    terminal_shell_pid: Option<u32>,

    /// When true, disables actual IPC communication and uses only local logging.
    /// Used during unit testing to avoid requiring a running VSCode extension.
    /// Set to false in production to enable real IPC communication with VSCode.
    test_mode: bool,
}



impl IPCCommunicator {
    pub async fn new(
        shell_pid: Option<u32>,
        reference_handle: crate::actor::ReferenceHandle,
        options: crate::Options,
    ) -> Result<Self> {
        info!("Creating IPC communicator for shell PID {shell_pid:?}");

        // Create actor system alongside existing connection management
        let dispatch_handle = {
            // Create client connection to daemon
            let (to_daemon_tx, from_daemon_rx) = crate::actor::spawn_client(
                DAEMON_SOCKET_PREFIX,
                true,                    // auto_start daemon
                "mcp-server",           // identity prefix
                options,                 // pass options for daemon spawning
            );

            // Create dispatch actor with client channels
            crate::actor::DispatchHandle::new(from_daemon_rx, to_daemon_tx, shell_pid, reference_handle)
        };

        Ok(Self {
            dispatch_handle,
            terminal_shell_pid: shell_pid,
            test_mode: false,
        })
    }

    /// Creates a new IPCCommunicator in test mode
    /// In test mode, all IPC operations are mocked and only local logging occurs
    pub fn new_test(_reference_handle: crate::actor::ReferenceHandle) -> Self {
        let mock_fn = Box::new(
            |mut _rx: tokio::sync::mpsc::Receiver<crate::types::IPCMessage>,
             _tx: tokio::sync::mpsc::Sender<crate::types::IPCMessage>| {
                Box::pin(async move {
                    // Minimal mock for test constructor
                })
                    as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            },
        ) as crate::actor::dispatch::MockActorFn;

        Self {
            dispatch_handle: crate::actor::dispatch::DispatchHandle::spawn_with_mock(mock_fn),
            terminal_shell_pid: None,
            test_mode: true,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if self.test_mode {
            info!("IPC Communicator initialized (test mode) - creating mock actor");

            // Create mock actor that responds to common messages
            let mock_fn = Box::new(
                |mut rx: tokio::sync::mpsc::Receiver<crate::types::IPCMessage>,
                 tx: tokio::sync::mpsc::Sender<crate::types::IPCMessage>| {
                    Box::pin(async move {
                        while let Some(message) = rx.recv().await {
                            use crate::types::IPCMessageType;

                            // Generate mock responses based on message type
                            match message.message_type {
                                IPCMessageType::TaskspaceState => {
                                    let response = crate::types::IPCMessage {
                                        message_type: crate::types::IPCMessageType::Response,
                                        id: uuid::Uuid::new_v4().to_string(),
                                        sender: message.sender.clone(),
                                        payload: serde_json::to_value(
                                            crate::types::TaskspaceStateResponse {
                                                name: Some("Mock Taskspace".to_string()),
                                                description: Some(
                                                    "Mock taskspace description".to_string(),
                                                ),
                                                initial_prompt: Some(
                                                    "Mock initial prompt".to_string(),
                                                ),
                                                collaborator: Some("sparkle".to_string()),
                                            },
                                        )
                                        .unwrap(),
                                    };
                                    let _ = tx.send(response).await;
                                }
                                IPCMessageType::PresentWalkthrough => {
                                    // Send acknowledgment for walkthrough
                                    let response = crate::types::IPCMessage {
                                        message_type: crate::types::IPCMessageType::Response,
                                        id: uuid::Uuid::new_v4().to_string(),
                                        sender: message.sender.clone(),
                                        payload: serde_json::to_value(()).unwrap(),
                                    };
                                    let _ = tx.send(response).await;
                                }
                                _ => {
                                    // For fire-and-forget messages, just log
                                    tracing::info!(
                                        "Mock actor received message: {:?}",
                                        message.message_type
                                    );
                                }
                            }
                        }
                    })
                        as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                },
            ) as crate::actor::dispatch::MockActorFn;

            self.dispatch_handle = crate::actor::dispatch::DispatchHandle::spawn_with_mock(mock_fn);
            return Ok(());
        }

        info!("IPC Communicator initialized with actor system");
        Ok(())
    }

    pub async fn present_walkthrough(
        &self,
        walkthrough: crate::ide::ResolvedWalkthrough,
    ) -> Result<()> {
        if self.test_mode {
            info!("Present walkthrough called (test mode): {:?}", walkthrough);
            return Ok(());
        }

        // Use new actor-based dispatch system
        let walkthrough_message = crate::types::PresentWalkthroughMessage {
            content: walkthrough.content,
            base_uri: walkthrough.base_uri,
        };
        let _response: () = self
            .dispatch_handle
            .send(walkthrough_message)
            .await
            .map_err(|e| {
                IPCError::SendError(format!(
                    "Failed to send present_walkthrough via actors: {}",
                    e
                ))
            })?;
        info!("Successfully presented walkthrough to VSCode via actor system");
        Ok(())
    }

    pub async fn get_selection(&self) -> Result<GetSelectionResult> {
        if self.test_mode {
            info!("Get selection called (test mode)");
            return Ok(GetSelectionResult {
                selected_text: None,
                file_path: None,
                start_line: None,
                start_column: None,
                end_line: None,
                end_column: None,
                line_number: None,
                document_language: None,
                is_untitled: None,
                message: Some("No selection available (test mode)".to_string()),
            });
        }

        // Use actor dispatch system for get_selection request/reply
        let get_selection_message = GetSelectionMessage {};
        let selection: GetSelectionResult = self
            .dispatch_handle
            .send(get_selection_message)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send get_selection via actors: {}", e))
            })?;

        info!("Successfully retrieved selection via actor system");
        Ok(selection)
    }

    /// Sends a log message out over the IPC bus
    pub async fn send_log_message(&self, level: LogLevel, message: String) {
        // In test mode, only do local logging
        if self.test_mode {
            return;
        }

        // Dispatch log over the IPC bus to get a central record
        let log_message = crate::types::LogMessage { level, message };
        if let Err(e) = self.dispatch_handle.send(log_message).await {
            // If IPC fails, we still have local logging above
            debug!("Failed to send log via actor dispatch: {}", e);
        }
    }

    /// Send Polo discovery message (MCP server announces presence with shell PID)
    pub async fn send_polo(&self) -> Result<()> {
        if self.test_mode {
            info!(
                "Polo discovery message sent (test mode) with shell PID: {:?}",
                self.terminal_shell_pid
            );
            return Ok(());
        }

        // Use new actor-based dispatch system
        // Note: PoloMessage payload is empty; shell_pid is in MessageSender
        let polo_message = crate::types::PoloMessage {};
        self.dispatch_handle
            .send(polo_message)
            .await
            .map_err(|e| IPCError::SendError(format!("Failed to send Polo via actors: {}", e)))?;
        info!(
            "Polo discovery message sent via actor system with shell PID: {:?}",
            self.terminal_shell_pid
        );
        Ok(())
    }

    /// Send Goodbye discovery message (MCP server announces departure with shell PID)
    pub async fn send_goodbye(&self) -> Result<()> {
        if self.test_mode {
            info!(
                "Goodbye discovery message sent (test mode) with shell PID: {:?}",
                self.terminal_shell_pid
            );
            return Ok(());
        }

        // Use new actor-based dispatch system
        // Note: GoodbyePayload is empty; shell_pid is in MessageSender
        let goodbye_payload = crate::types::GoodbyePayload {};
        self.dispatch_handle
            .send(goodbye_payload)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send Goodbye via actors: {}", e))
            })?;
        info!(
            "Goodbye discovery message sent via actor system with shell PID: {:?}",
            self.terminal_shell_pid
        );
        Ok(())
    }

    /// Send spawn_taskspace message to create new taskspace
    pub async fn spawn_taskspace(
        &self,
        name: String,
        task_description: String,
        initial_prompt: String,
        collaborator: Option<String>,
    ) -> Result<()> {
        if self.test_mode {
            info!("Spawn taskspace called (test mode): {}", name);
            return Ok(());
        }

        // Use new actor-based dispatch system
        let (project_path, taskspace_uuid) = extract_project_info()?;
        let spawn_payload = crate::types::SpawnTaskspacePayload {
            project_path,
            taskspace_uuid,
            name,
            task_description,
            initial_prompt,
            collaborator,
        };
        self.dispatch_handle
            .send(spawn_payload)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send spawn_taskspace via actors: {}", e))
            })?;
        Ok(())
    }

    /// Send log_progress message to report agent progress
    pub async fn log_progress(
        &self,
        message: String,
        category: crate::types::ProgressCategory,
    ) -> Result<()> {
        if self.test_mode {
            info!(
                "Log progress called (test mode): {} - {:?}",
                message, category
            );
            return Ok(());
        }

        // Use new actor-based dispatch system
        let (project_path, taskspace_uuid) = extract_project_info()?;
        let progress_payload = crate::types::LogProgressPayload {
            project_path,
            taskspace_uuid,
            message,
            category,
        };
        self.dispatch_handle
            .send(progress_payload)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send log_progress via actors: {}", e))
            })?;
        return Ok(());
    }

    /// Send signal_user message to request user attention
    pub async fn signal_user(&self, message: String) -> Result<()> {
        if self.test_mode {
            info!("Signal user called (test mode): {}", message);
            return Ok(());
        }

        // Use new actor-based dispatch system
        let (project_path, taskspace_uuid) = extract_project_info()?;
        let signal_payload = crate::types::SignalUserPayload {
            project_path,
            taskspace_uuid,
            message,
        };
        self.dispatch_handle
            .send(signal_payload)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send signal_user via actors: {}", e))
            })?;
        return Ok(());
    }

    /// Send update_taskspace message to update taskspace metadata
    pub async fn update_taskspace(
        &self,
        name: String,
        description: String,
        collaborator: Option<String>,
    ) -> Result<crate::types::TaskspaceStateResponse> {
        let (project_path, taskspace_uuid) = extract_project_info()?;

        // Use actor dispatch system for update_taskspace request/reply
        let request = crate::types::TaskspaceStateRequest {
            project_path,
            taskspace_uuid,
            name: Some(name),
            description: Some(description),
            collaborator,
        };
        let response: crate::types::TaskspaceStateResponse =
            self.dispatch_handle.send(request).await.map_err(|e| {
                IPCError::SendError(format!("Failed to update taskspace via actors: {}", e))
            })?;
        Ok(response)
    }

    /// Get current taskspace state from the Symposium daemon/app
    ///
    /// This is a key method in the dynamic agent initialization system. It enables
    /// the MCP server to retrieve real taskspace information (name, description,
    /// initial_prompt) which gets included in the `/yiasou` prompt for agent boot.
    ///
    /// **System Role:**
    /// - Called by `get_taskspace_context()` in server.rs during prompt assembly
    /// - Bridges MCP server ↔ Symposium daemon ↔ Symposium app communication
    /// - Enables dynamic, context-aware agent initialization vs static prompts
    ///
    /// **Field Semantics:**
    /// - `name`: User-visible taskspace name (GUI display)
    /// - `description`: User-visible summary (GUI tooltips, etc.)
    /// - `initial_prompt`: LLM task description (cleared after agent startup)
    ///
    /// **Lifecycle Integration:**
    /// - First call: Returns initial_prompt for agent initialization
    /// - After update_taskspace: GUI app clears initial_prompt (natural cleanup)
    ///
    /// **Flow:**
    /// 1. Extract taskspace UUID from current directory structure
    /// 2. Send GetTaskspaceState IPC message to daemon with project/taskspace info
    /// 3. Daemon forwards request to Symposium app
    /// 4. App returns current taskspace state (name, description, initial_prompt)
    /// 5. Response flows back through daemon to MCP server
    ///
    /// **Error Handling:**
    /// - If taskspace detection fails → extract_project_info() error
    /// - If daemon unreachable → IPC timeout/connection error  
    /// - If app unavailable → daemon returns empty/error response
    /// - Caller (get_taskspace_context) handles errors gracefully
    pub async fn get_taskspace_state(&self) -> Result<crate::types::TaskspaceStateResponse> {
        // Extract taskspace UUID from directory structure (task-UUID/.symposium pattern)
        let (project_path, taskspace_uuid) = extract_project_info()?;

        // Use new actor-based dispatch system
        let request = crate::types::TaskspaceStateRequest {
            project_path,
            taskspace_uuid,
            name: None,
            description: None,
            collaborator: None,
        };
        let response: crate::types::TaskspaceStateResponse =
            self.dispatch_handle.send(request).await.map_err(|e| {
                IPCError::SendError(format!("Failed to get taskspace state via actors: {}", e))
            })?;
        return Ok(response);
    }

    /// Send delete_taskspace message to delete current taskspace
    pub async fn delete_taskspace(&self) -> Result<()> {
        if self.test_mode {
            info!("Delete taskspace called (test mode)");
            return Ok(());
        }

        // Use new actor-based dispatch system
        let (project_path, taskspace_uuid) = extract_project_info()?;
        let delete_payload = crate::types::DeleteTaskspacePayload {
            project_path,
            taskspace_uuid,
        };
        self.dispatch_handle
            .send(delete_payload)
            .await
            .map_err(|e| {
                IPCError::SendError(format!("Failed to send delete_taskspace via actors: {}", e))
            })?;
        return Ok(());
    }

    /// Gracefully shutdown the IPC communicator, sending Goodbye discovery message
    pub async fn shutdown(&self) -> Result<()> {
        if self.test_mode {
            info!("IPC shutdown (test mode)");
            return Ok(());
        }

        self.send_goodbye().await?;
        info!("Sent Goodbye discovery message during shutdown");
        Ok(())
    }
}


// Implementation of IpcClient trait for IDE operations
// Implementation of IpcClient trait for IDE operations
impl crate::ide::IpcClient for IPCCommunicator {
    async fn resolve_symbol_by_name(
        &mut self,
        name: &str,
    ) -> anyhow::Result<Vec<crate::ide::SymbolDef>> {
        if self.test_mode {
            return Ok(vec![]);
        }

        let payload = ResolveSymbolByNamePayload {
            name: name.to_string(),
        };

        let symbols: Vec<crate::ide::SymbolDef> = self
            .dispatch_handle
            .send(payload)
            .await
            .with_context(|| format!("failed to resolve symbol '{name}'"))?;

        Ok(symbols)
    }

    async fn find_all_references(
        &mut self,
        symbol: &crate::ide::SymbolDef,
    ) -> anyhow::Result<Vec<crate::ide::FileRange>> {
        if self.test_mode {
            return Ok(vec![]);
        }

        let payload = FindAllReferencesPayload {
            symbol: symbol.clone(),
        };

        let locations: Vec<crate::ide::FileRange> =
            self.dispatch_handle.send(payload).await.with_context(|| {
                format!(
                    "VSCode extension failed to find references for symbol '{}'",
                    symbol.name
                )
            })?;

        Ok(locations)
    }

    fn generate_uuid(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod test {
    //! Integration tests for Dialectic MCP Server
    //!
    //! Tests the IPC communication layer and message structure

    use crate::ipc::IPCCommunicator;
    use crate::types::{
        IPCMessage, IPCMessageType, MessageSender, PresentReviewParams, ReviewMode,
    };
    use serde_json;

    #[tokio::test]
    async fn test_get_selection_test_mode() {
        let _ = tracing_subscriber::fmt::try_init();

        let reference_handle = crate::actor::ReferenceHandle::new();
        let ipc = IPCCommunicator::new_test(reference_handle);

        // Test get_selection in test mode
        let result = ipc.get_selection().await;
        assert!(result.is_ok());

        let selection_result = result.unwrap();
        assert!(selection_result.selected_text.is_none());
        assert!(selection_result.message.is_some());
        assert!(selection_result.message.unwrap().contains("test mode"));
    }

    #[tokio::test]
    async fn test_ipc_message_structure() {
        let _ = tracing_subscriber::fmt::try_init();

        // This test verifies that the IPC message structure is correct
        use uuid::Uuid;

        let params = PresentReviewParams {
            content: "# Review Content".to_string(),
            mode: ReviewMode::Append,
            section: None,
            base_uri: "/project/root".to_string(),
        };

        // Create an IPC message like the server would
        let message = IPCMessage {
            message_type: IPCMessageType::PresentReview,
            id: Uuid::new_v4().to_string(),
            sender: MessageSender {
                working_directory: "/project/root".to_string(),
                taskspace_uuid: None,
                shell_pid: Some(12345),
            },
            payload: serde_json::to_value(&params).unwrap(),
        };

        // Verify IPC message structure
        assert!(!message.id.is_empty());
        assert!(Uuid::parse_str(&message.id).is_ok());
        assert!(matches!(
            message.message_type,
            IPCMessageType::PresentReview
        ));
        assert!(message.payload.is_object());

        // Verify payload can be deserialized back to PresentReviewParams
        let deserialized: PresentReviewParams =
            serde_json::from_value(message.payload.clone()).unwrap();
        assert_eq!(deserialized.content, "# Review Content");
        assert!(matches!(deserialized.mode, ReviewMode::Append));
        assert_eq!(deserialized.base_uri, "/project/root");
    }
}
