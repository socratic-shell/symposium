//! IPC communication module for Dialectic MCP Server
//!
//! Handles Unix socket/named pipe communication with the VSCode extension.
//! Ports the logic from server/src/ipc.ts to Rust with cross-platform support.

use crate::types::{
    FindAllReferencesPayload, GetSelectionResult, GoodbyePayload, IPCMessage, IPCMessageType,
    LogLevel, LogParams, MessageSender, PoloPayload, ResolveSymbolByNamePayload, ResponsePayload,
};
use anyhow::Context;
use futures::FutureExt;
use serde::de::DeserializeOwned;
use serde_json;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::oneshot::Sender;
use tokio::sync::{Mutex, oneshot};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

/// Create MessageSender with current context information
///
/// Attempts to gather working directory, taskspace UUID, and shell PID for message routing.
/// Falls back gracefully when information is not available.
fn create_message_sender(shell_pid: Option<u32>) -> MessageSender {
    // Get working directory - always required
    let working_directory = std::env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default(); // empty string in case of error fetching current directory

    // Try to extract taskspace UUID from directory structure
    let taskspace_uuid = extract_project_info().map(|(_, uuid)| uuid).ok();

    MessageSender {
        working_directory,
        taskspace_uuid,
        shell_pid,
    }
}

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
/// Currently in transition: uses both legacy connection management and new actor system.
/// Marco/Polo messages use actors, other messages use legacy system during migration.
#[derive(Clone)]
pub struct IPCCommunicator {
    inner: Arc<Mutex<IPCCommunicatorInner>>,
    reference_store: Arc<crate::reference_store::ReferenceStore>,
    
    /// New actor-based dispatch system (for marco/polo messages initially)
    dispatch_handle: crate::actor::DispatchHandle,

    /// When true, disables actual IPC communication and uses only local logging.
    /// Used during unit testing to avoid requiring a running VSCode extension.
    /// Set to false in production to enable real IPC communication with VSCode.
    test_mode: bool,
}

struct IPCCommunicatorInner {
    /// Write half of the Unix socket connection to VSCode extension
    write_half: Option<Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>>,

    /// Tracks outgoing requests awaiting responses from VSCode extension
    /// Key: unique message ID (UUID), Value: channel to send response back to caller
    /// Enables concurrent request/response handling with proper correlation
    pending_requests: HashMap<String, oneshot::Sender<ResponsePayload>>,

    /// Flag to track if we have an active connection and reader task
    /// When true, ensure_connection() is a no-op
    connected: bool,

    /// Terminal shell PID for this MCP server instance
    /// Reported to extension during handshake for smart terminal selection
    terminal_shell_pid: u32,
}

impl IPCCommunicator {
    pub async fn new(
        shell_pid: u32,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Result<Self> {
        info!("Creating IPC communicator for shell PID {shell_pid}");

        // Create actor system alongside existing connection management
        let dispatch_handle = {
            // Create client connection to daemon
            let (to_daemon_tx, from_daemon_rx) = crate::actor::spawn_client(
                "dialectic".to_string(), // socket prefix
                true, // auto_start daemon
            );

            // Create dispatch actor with client channels
            crate::actor::DispatchHandle::new(from_daemon_rx, to_daemon_tx)
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(IPCCommunicatorInner {
                write_half: None,
                pending_requests: HashMap::new(),
                connected: false,
                terminal_shell_pid: shell_pid,
            })),
            reference_store,
            dispatch_handle,
            test_mode: false,
        })
    }

    /// Creates a new IPCCommunicator in test mode
    /// In test mode, all IPC operations are mocked and only local logging occurs
    pub fn new_test(reference_store: Arc<crate::reference_store::ReferenceStore>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(IPCCommunicatorInner {
                write_half: None,
                pending_requests: HashMap::new(),
                connected: false,
                terminal_shell_pid: 0, // Dummy PID for test mode
            })),
            reference_store,
            dispatch_handle: None, // No actors in test mode for now
            test_mode: true,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        if self.test_mode {
            info!("IPC Communicator initialized (test mode) - creating mock actor");
            
            // Create mock actor that responds to common messages
            let mock_fn = Box::new(|mut rx: tokio::sync::mpsc::Receiver<crate::types::IPCMessage>, tx: tokio::sync::mpsc::Sender<crate::types::IPCMessage>| {
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
                                    payload: serde_json::to_value(crate::types::TaskspaceStateResponse {
                                        name: Some("Mock Taskspace".to_string()),
                                        description: Some("Mock taskspace description".to_string()),
                                        initial_prompt: Some("Mock initial prompt".to_string()),
                                    }).unwrap(),
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
                                tracing::info!("Mock actor received message: {:?}", message.message_type);
                            }
                        }
                    }
                }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            }) as crate::actor::dispatch::MockActorFn;
            
            self.dispatch_handle = Some(crate::actor::dispatch::DispatchHandle::spawn_with_mock(mock_fn));
            return Ok(());
        }

        // Use ensure_connection for initial connection (legacy system)
        IPCCommunicatorInner::ensure_connection(
            Arc::clone(&self.inner),
            Arc::clone(&self.reference_store),
        )
        .await?;

        info!("Connected to message bus daemon via IPC (legacy + actor systems active)");
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
        let _response: () = self.dispatch_handle.send(walkthrough_message).await
            .map_err(|e| IPCError::SendError(format!("Failed to send present_walkthrough via actors: {}", e)))?;
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

        // Ensure connection is established before proceeding
        IPCCommunicatorInner::ensure_connection(
            Arc::clone(&self.inner),
            Arc::clone(&self.reference_store),
        )
        .await?;

        // Create message payload with shell PID for multi-window filtering
        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let message = IPCMessage {
            message_type: IPCMessageType::GetSelection,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::json!({}),
        };

        debug!("Sending get_selection message: {:?}", message);

        let selection: GetSelectionResult = self.send_message_with_reply(message).await?;
        Ok(selection)
    }

    pub async fn send_log(&self, level: LogLevel, message: String) {
        // Always log locally using Rust logging infrastructure
        match level {
            LogLevel::Info => info!("{}", message),
            LogLevel::Error => error!("{}", message),
            LogLevel::Debug => debug!("{}", message),
        }

        // In test mode, only do local logging
        if self.test_mode {
            return;
        }

        // Use new actor-based dispatch system
        let log_message = crate::types::LogMessage { level, message };
        if let Err(e) = self.dispatch_handle.send(log_message).await {
            // If IPC fails, we still have local logging above
            debug!("Failed to send log via actor dispatch: {}", e);
        }
    }

    /// Send Polo discovery message (MCP server announces presence with shell PID)
    pub async fn send_polo(&self, terminal_shell_pid: u32) -> Result<()> {
        if self.test_mode {
            info!(
                "Polo discovery message sent (test mode) with shell PID: {}",
                terminal_shell_pid
            );
            return Ok(());
        }

        // Use new actor-based dispatch system
        let polo_message = crate::types::PoloMessage { terminal_shell_pid };
        self.dispatch_handle.send(polo_message).await
            .map_err(|e| IPCError::SendError(format!("Failed to send Polo via actors: {}", e)))?;
        info!("Polo discovery message sent via actor system with shell PID: {}", terminal_shell_pid);
        Ok(())
    }

    /// Send Goodbye discovery message (MCP server announces departure with shell PID)
    pub async fn send_goodbye(&self, terminal_shell_pid: u32) -> Result<()> {
        if self.test_mode {
            info!(
                "Goodbye discovery message sent (test mode) with shell PID: {}",
                terminal_shell_pid
            );
            return Ok(());
        }

        // Use new actor-based dispatch system
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let goodbye_payload = crate::types::GoodbyePayload {};
            dispatch_handle.send(goodbye_payload).await
                .map_err(|e| IPCError::SendError(format!("Failed to send Goodbye via actors: {}", e)))?;
            info!("Goodbye discovery message sent via actor system with shell PID: {}", terminal_shell_pid);
            return Ok(());
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy Goodbye sending");
        let payload = GoodbyePayload {};
        let message = IPCMessage {
            message_type: IPCMessageType::Goodbye,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(Some(terminal_shell_pid)),
            payload: serde_json::to_value(payload)?,
        };

        debug!(
            "Sending Goodbye discovery message with shell PID: {}",
            terminal_shell_pid
        );
        self.send_message_without_reply(message).await
    }

    /// Send spawn_taskspace message to create new taskspace
    pub async fn spawn_taskspace(
        &self,
        name: String,
        task_description: String,
        initial_prompt: String,
    ) -> Result<()> {
        if self.test_mode {
            info!("Spawn taskspace called (test mode): {}", name);
            return Ok(());
        }

        // Use new actor-based dispatch system
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let (project_path, taskspace_uuid) = extract_project_info()?;
            let spawn_payload = crate::types::SpawnTaskspacePayload {
                project_path,
                taskspace_uuid,
                name,
                task_description,
                initial_prompt,
            };
            dispatch_handle.send(spawn_payload).await
                .map_err(|e| IPCError::SendError(format!("Failed to send spawn_taskspace via actors: {}", e)))?;
            return Ok(());
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy spawn_taskspace sending");
        
        use crate::types::{IPCMessageType, SpawnTaskspacePayload};

        let (project_path, taskspace_uuid) = extract_project_info()?;

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let message = IPCMessage {
            message_type: IPCMessageType::SpawnTaskspace,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(SpawnTaskspacePayload {
                project_path,
                taskspace_uuid,
                name,
                task_description,
                initial_prompt,
            })?,
        };

        self.send_message_without_reply(message).await
    }

    /// Send log_progress message to report agent progress
    pub async fn log_progress(
        &self,
        message: String,
        category: crate::types::ProgressCategory,
    ) -> Result<()> {
        if self.test_mode {
            info!("Log progress called (test mode): {} - {:?}", message, category);
            return Ok(());
        }

        // Use new actor-based dispatch system
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let (project_path, taskspace_uuid) = extract_project_info()?;
            let progress_payload = crate::types::LogProgressPayload {
                project_path,
                taskspace_uuid,
                message,
                category,
            };
            dispatch_handle.send(progress_payload).await
                .map_err(|e| IPCError::SendError(format!("Failed to send log_progress via actors: {}", e)))?;
            return Ok(());
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy log_progress sending");
        
        use crate::types::{IPCMessageType, LogProgressPayload};

        let (project_path, taskspace_uuid) = extract_project_info()?;

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let ipc_message = IPCMessage {
            message_type: IPCMessageType::LogProgress,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(LogProgressPayload {
                project_path,
                taskspace_uuid,
                message,
                category,
            })?,
        };

        self.send_message_without_reply(ipc_message).await
    }

    /// Send signal_user message to request user attention
    pub async fn signal_user(&self, message: String) -> Result<()> {
        if self.test_mode {
            info!("Signal user called (test mode): {}", message);
            return Ok(());
        }

        // Use new actor-based dispatch system
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let (project_path, taskspace_uuid) = extract_project_info()?;
            let signal_payload = crate::types::SignalUserPayload {
                project_path,
                taskspace_uuid,
                message,
            };
            dispatch_handle.send(signal_payload).await
                .map_err(|e| IPCError::SendError(format!("Failed to send signal_user via actors: {}", e)))?;
            return Ok(());
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy signal_user sending");
        
        use crate::types::{IPCMessageType, SignalUserPayload};

        let (project_path, taskspace_uuid) = extract_project_info()?;

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let ipc_message = IPCMessage {
            message_type: IPCMessageType::SignalUser,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(SignalUserPayload {
                project_path,
                taskspace_uuid,
                message,
            })?,
        };

        self.send_message_without_reply(ipc_message).await
    }

    /// Send update_taskspace message to update taskspace metadata
    pub async fn update_taskspace(
        &self,
        name: String,
        description: String,
    ) -> Result<crate::types::TaskspaceStateResponse> {
        use crate::types::{IPCMessageType, TaskspaceStateRequest, TaskspaceStateResponse};
        let (project_path, taskspace_uuid) = extract_project_info()?;

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let ipc_message = IPCMessage {
            message_type: IPCMessageType::TaskspaceState,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(TaskspaceStateRequest {
                project_path,
                taskspace_uuid,
                name: Some(name),
                description: Some(description),
            })?,
        };

        let taskspace_state: TaskspaceStateResponse =
            self.send_message_with_reply(ipc_message).await?;
        Ok(taskspace_state)
    }

    /// Fetch current taskspace state from the Symposium daemon/app
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
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let request = crate::types::TaskspaceStateRequest {
                project_path,
                taskspace_uuid,
                name: None,
                description: None,
            };
            let response: crate::types::TaskspaceStateResponse = dispatch_handle.send(request).await
                .map_err(|e| IPCError::SendError(format!("Failed to get taskspace state via actors: {}", e)))?;
            return Ok(response);
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy get_taskspace_state");
        
        use crate::types::{IPCMessageType, TaskspaceStateRequest, TaskspaceStateResponse};

        // Get our shell PID for message routing
        let shell_pid = {
            let inner = self.inner.lock().await;
            inner.terminal_shell_pid
        };

        // Construct IPC message requesting taskspace state
        let ipc_message = IPCMessage {
            message_type: IPCMessageType::TaskspaceState,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(Some(shell_pid)),
            payload: serde_json::to_value(TaskspaceStateRequest {
                project_path,
                taskspace_uuid,
                name: None,
                description: None,
            })?,
        };

        // Send message and wait for response from daemon/app
        let taskspace_state: TaskspaceStateResponse =
            self.send_message_with_reply(ipc_message).await?;
        Ok(taskspace_state)
    }

    /// Send delete_taskspace message to delete current taskspace
    pub async fn delete_taskspace(&self) -> Result<()> {
        if self.test_mode {
            info!("Delete taskspace called (test mode)");
            return Ok(());
        }

        // Use new actor-based dispatch system
        if let Some(dispatch_handle) = &self.dispatch_handle {
            let (project_path, taskspace_uuid) = extract_project_info()?;
            let delete_payload = crate::types::DeleteTaskspacePayload {
                project_path,
                taskspace_uuid,
            };
            dispatch_handle.send(delete_payload).await
                .map_err(|e| IPCError::SendError(format!("Failed to send delete_taskspace via actors: {}", e)))?;
            return Ok(());
        }

        // Fallback to legacy system (should not happen in current setup)
        warn!("No dispatch handle available, using legacy delete_taskspace sending");
        
        use crate::types::{DeleteTaskspacePayload, IPCMessageType};

        let (project_path, taskspace_uuid) = extract_project_info()?;

        let shell_pid = {
            let inner = self.inner.lock().await;
            inner.terminal_shell_pid
        };

        let ipc_message = IPCMessage {
            message_type: IPCMessageType::DeleteTaskspace,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(Some(shell_pid)),
            payload: serde_json::to_value(DeleteTaskspacePayload {
                project_path,
                taskspace_uuid,
            })?,
        };

        self.send_message_without_reply(ipc_message).await
    }

    /// Gracefully shutdown the IPC communicator, sending Goodbye discovery message
    pub async fn shutdown(&self) -> Result<()> {
        if self.test_mode {
            info!("IPC shutdown (test mode)");
            return Ok(());
        }

        let shell_pid = {
            let inner_guard = self.inner.lock().await;
            inner_guard.terminal_shell_pid
        };

        self.send_goodbye(shell_pid).await?;
        info!("Sent Goodbye discovery message during shutdown");
        Ok(())
    }

    /// Sends an IPC message and waits for a response from VSCode extension
    ///
    /// Sets up response correlation using the message UUID and waits for response.
    /// Uses the underlying `write_message` primitive to send the data.
    async fn send_message_with_reply<R>(&self, message: IPCMessage) -> Result<R>
    where
        R: DeserializeOwned,
    {
        // Use standard timeout for all messages
        let timeout_duration = std::time::Duration::from_secs(5);
        debug!(
            "Sending IPC message with ID: {} (PID: {})",
            message.id,
            std::process::id()
        );

        let (tx, rx) = oneshot::channel();

        // Store the response channel
        {
            let mut inner = self.inner.lock().await;
            trace!("Storing response channel for message ID: {}", message.id);
            inner.pending_requests.insert(message.id.clone(), tx);
            trace!("Pending requests count: {}", inner.pending_requests.len());
        }

        // Send the message
        let message_data = serde_json::to_string(&message)?;
        trace!("Serialized message data: {}", message_data);
        trace!("About to call write_message");

        self.write_message(&message_data).await?;
        trace!("write_message completed successfully");

        trace!("Waiting for response with 5 second timeout...");

        // Wait for response with appropriate timeout
        let response = tokio::time::timeout(timeout_duration, rx)
            .await
            .map_err(|_| {
                // Clean up the leaked entry on timeout to fix memory leak
                let inner_clone = Arc::clone(&self.inner);
                let message_id = message.id.clone();
                tokio::spawn(async move {
                    let mut inner = inner_clone.lock().await;
                    inner.pending_requests.remove(&message_id);
                });
                error!("Timeout waiting for response to message ID: {}", message.id);
                IPCError::Timeout
            })?
            .map_err(|_| IPCError::ChannelClosed)?;

        // Parse UserFeedback from response data
        let user_feedback: R = if let Some(data) = response.data {
            serde_json::from_value(data).map_err(IPCError::SerializationError)?
        } else {
            serde_json::from_value(serde_json::Value::Null)?
        };
        Ok(user_feedback)
    }

    /// Sends an IPC message without waiting for a response (fire-and-forget)
    ///
    /// Used for operations like logging where we don't need confirmation from VSCode.
    /// Uses the underlying `write_message` primitive to send the data.
    async fn send_message_without_reply(&self, message: IPCMessage) -> Result<()> {
        let message_data = serde_json::to_string(&message)?;
        self.write_message(&message_data).await
    }

    /// Low-level primitive for writing raw JSON data to the IPC connection (Unix)
    ///
    /// This is the underlying method used by both `send_message_with_reply` and
    /// `send_message_without_reply`. It handles the platform-specific socket writing
    /// and adds newline delimiters for message boundaries.
    ///
    /// ## Known Edge Case: Write Failure Race Condition
    ///
    /// There's a rare race condition where the extension restarts between the time
    /// `ensure_connection()` checks `connected: true` and when this method attempts
    /// to write. In this case:
    ///
    /// 1. `ensure_connection()` sees stale `connected: true` state (reader hasn't detected failure yet)
    /// 2. `write_message()` fails with "Broken pipe" or similar
    /// 3. Error is returned to user (operation fails)
    /// 4. Reader task detects failure and reconnects in background
    /// 5. User's retry succeeds
    ///
    /// This is acceptable because:
    /// - The race window is very small (reader task detects failures quickly)
    /// - The failure is transient and self-healing
    /// - Multiple recovery mechanisms provide eventual consistency
    /// - Adding write error recovery would significantly complicate the code
    async fn write_message(&self, data: &str) -> Result<()> {
        trace!("write_message called with data length: {}", data.len());

        let inner = self.inner.lock().await;
        if let Some(ref write_half) = inner.write_half {
            trace!("Got write half, writing to Unix socket");
            let mut writer = write_half.lock().await;

            trace!("Writing message data to socket");
            writer.write_all(data.as_bytes()).await?;

            trace!("Writing newline delimiter");
            writer.write_all(b"\n").await?; // Add newline delimiter

            trace!("write_message completed successfully");
            Ok(())
        } else {
            error!("write_message called but no connection available");
            Err(IPCError::NotConnected)
        }
    }
}

impl IPCCommunicatorInner {
    /// Ensures connection is established, connecting if necessary
    /// Idempotent - safe to call multiple times, only connects if not already connected
    async fn ensure_connection(
        this: Arc<Mutex<Self>>,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Result<()> {
        let mut inner = this.lock().await;
        if inner.connected {
            return Ok(()); // Already connected, nothing to do
        }

        inner
            .attempt_connection_with_backoff(&this, reference_store)
            .await
    }

    /// Clears dead connection state and attempts fresh reconnection
    /// Called by reader task as "parting gift" when connection dies
    async fn clear_connection_and_reconnect(
        this: Arc<Mutex<Self>>,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) {
        info!("Clearing dead connection state and attempting reconnection");

        let mut inner = this.lock().await;

        // Clean up dead connection state
        inner.connected = false;
        inner.write_half = None;

        // Clean up orphaned pending requests to fix memory leak
        let orphaned_count = inner.pending_requests.len();
        if orphaned_count > 0 {
            warn!("Cleaning up {} orphaned pending requests", orphaned_count);
            inner.pending_requests.clear();
        }

        // Attempt fresh connection
        match inner
            .attempt_connection_with_backoff(&this, reference_store)
            .await
        {
            Ok(()) => {
                info!("Reader task successfully reconnected");
            }
            Err(e) => {
                error!("Reader task failed to reconnect: {}", e);
                info!("Next MCP operation will retry connection");
            }
        }
    }

    /// Attempts connection with exponential backoff to handle extension restart timing
    ///
    /// Runs while holding the lock to avoid races where multiple concurrent attempts
    /// try to re-establish the connection. This ensures only one connection attempt
    /// happens at a time, preventing duplicate reader tasks or connection state corruption.
    async fn attempt_connection_with_backoff(
        &mut self,
        this: &Arc<Mutex<Self>>,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Result<()> {
        // Precondition: we should only be called when disconnected
        assert!(
            !self.connected,
            "attempt_connection_with_backoff called while already connected"
        );
        assert!(
            self.write_half.is_none(),
            "attempt_connection_with_backoff called with existing write_half"
        );

        const MAX_RETRIES: u32 = 5;
        const BASE_DELAY_MS: u64 = 100;

        let socket_path =
            crate::constants::daemon_socket_path(crate::constants::DAEMON_SOCKET_PREFIX);
        info!(
            "Attempting connection to message bus daemon: {}",
            socket_path
        );

        for attempt in 1..=MAX_RETRIES {
            match UnixStream::connect(&socket_path).await {
                Ok(stream) => {
                    info!("Successfully connected on attempt {}", attempt);

                    // Split the stream into read and write halves
                    let (read_half, write_half) = stream.into_split();
                    let write_half = Arc::new(Mutex::new(write_half));

                    // Update connection state (we already hold the lock)
                    self.write_half = Some(Arc::clone(&write_half));
                    self.connected = true;

                    // Spawn new reader task with cloned Arc
                    let inner_clone = Arc::clone(this);
                    let reference_store_clone = Arc::clone(&reference_store);
                    tokio::spawn(async move {
                        IPCCommunicator::response_reader_task(
                            read_half,
                            inner_clone,
                            reference_store_clone,
                        )
                        .await;
                    });

                    return Ok(());
                }
                Err(e) if attempt < MAX_RETRIES => {
                    let delay = Duration::from_millis(BASE_DELAY_MS * 2_u64.pow(attempt - 1));
                    warn!(
                        "Connection attempt {} failed: {}. Retrying in {:?}",
                        attempt, e, delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    error!("All connection attempts failed. Last error: {}", e);
                    return Err(IPCError::ConnectionFailed {
                        path: socket_path,
                        source: e,
                    }
                    .into());
                }
            }
        }

        unreachable!("Loop should always return or error")
    }
}

impl IPCCommunicator {
    fn response_reader_task(
        mut read_half: tokio::net::unix::OwnedReadHalf,
        inner: Arc<Mutex<IPCCommunicatorInner>>,
        reference_store: Arc<crate::reference_store::ReferenceStore>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        async move {
            info!("Starting IPC response reader task (Unix)");

            let mut reader = BufReader::new(&mut read_half);

            loop {
                let mut buffer = Vec::new();

                trace!("response_reader_task: About to read from connection");

                // Read a line from the connection
                let read_result = reader.read_until(b'\n', &mut buffer).await;

                match read_result {
                    Ok(0) => {
                        warn!("IPC connection closed by VSCode extension");
                        break;
                    }
                    Ok(_) => {
                        // Remove the newline delimiter
                        if buffer.ends_with(&[b'\n']) {
                            buffer.pop();
                        }

                        let message_str = match String::from_utf8(buffer) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Received invalid UTF-8 from VSCode extension: {}", e);
                                continue;
                            }
                        };

                        Self::handle_incoming_message(&inner, &message_str, &reference_store).await;
                    }
                    Err(e) => {
                        error!("Error reading from IPC connection: {}", e);
                        break;
                    }
                }
            }

            // Reader task's "parting gift" - attempt reconnection before terminating
            info!("Reader task attempting reconnection as parting gift...");

            // Spawn the reconnection attempt to avoid blocking reader task termination
            let inner_for_reconnect = Arc::clone(&inner);
            let reference_store_for_reconnect = Arc::clone(&reference_store);
            tokio::spawn(IPCCommunicatorInner::clear_connection_and_reconnect(
                inner_for_reconnect,
                reference_store_for_reconnect,
            ));

            info!("IPC response reader task terminated");
        }
        .boxed()
    }

    /// Processes incoming messages from the daemon
    /// Handles both responses to our requests and incoming messages (like Marco)
    async fn handle_incoming_message(
        inner: &Arc<Mutex<IPCCommunicatorInner>>,
        message_str: &str,
        reference_store: &Arc<crate::reference_store::ReferenceStore>,
    ) {
        debug!(
            "Received IPC message (PID: {}): {}",
            std::process::id(),
            message_str
        );

        // Parse as unified IPCMessage
        let message: IPCMessage = match serde_json::from_str(message_str) {
            Ok(msg) => msg,
            Err(e) => {
                error!(
                    "Failed to parse incoming message: {} - Message: {}",
                    e, message_str
                );
                return;
            }
        };

        match message.message_type {
            IPCMessageType::Response => {
                // Handle response to our request
                let response_payload: ResponsePayload =
                    match serde_json::from_value(message.payload) {
                        Ok(payload) => payload,
                        Err(e) => {
                            error!("Failed to parse response payload: {}", e);
                            return;
                        }
                    };

                let mut inner_guard = inner.lock().await;
                if let Some(sender) = inner_guard.pending_requests.remove(&message.id) {
                    if let Err(_) = sender.send(response_payload) {
                        warn!("Failed to send response to caller - receiver dropped");
                    }
                } else {
                    // Every message (including the ones we send...) gets rebroadcast to everyone,
                    // so this is (hopefully) to some other MCP server. Just ignore it.
                    debug!(
                        "Received response for unknown request ID: {} (PID: {})",
                        message.id,
                        std::process::id()
                    );
                }
            }
            IPCMessageType::Marco => {
                info!("Received Marco discovery message, responding with Polo");

                // Get shell PID from inner state
                let shell_pid = {
                    let inner_guard = inner.lock().await;
                    inner_guard.terminal_shell_pid
                };

                // Create a temporary IPCCommunicator to send Polo response
                let temp_communicator = IPCCommunicator {
                    inner: Arc::clone(inner),
                    reference_store: Arc::clone(reference_store),
                    dispatch_handle: None, // No actors needed for legacy polo response
                    test_mode: false,
                };

                if let Err(e) = temp_communicator.send_polo(shell_pid).await {
                    error!("Failed to send Polo response to Marco: {}", e);
                }
            }
            IPCMessageType::StoreReference => {
                info!("Received store reference message");

                // Deserialize payload into StoreReferencePayload struct
                let payload: crate::types::StoreReferencePayload =
                    match serde_json::from_value(message.payload) {
                        Ok(payload) => payload,
                        Err(e) => {
                            error!("Failed to deserialize store_reference payload: {}", e);
                            return;
                        }
                    };

                // Store the arbitrary JSON value in the reference store
                match reference_store
                    .store_json_with_id(&payload.key, payload.value)
                    .await
                {
                    Ok(()) => {
                        info!("Successfully stored reference {}", payload.key);
                    }
                    Err(e) => {
                        error!("Failed to store reference {}: {}", payload.key, e);
                    }
                }
            }
            _ => {
                // Every message (including the ones we send...) gets rebroadcast to everyone,
                // so we can just ignore anything else.
            }
        }
    }
}

// Implementation of IpcClient trait for IDE operations
impl crate::ide::IpcClient for IPCCommunicator {
    async fn resolve_symbol_by_name(
        &mut self,
        name: &str,
    ) -> anyhow::Result<Vec<crate::ide::SymbolDef>> {
        if self.test_mode {
            // Return empty result in test mode
            return Ok(vec![]);
        }

        let payload = ResolveSymbolByNamePayload {
            name: name.to_string(),
        };

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let message = IPCMessage {
            message_type: IPCMessageType::ResolveSymbolByName,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(payload)?,
        };

        let symbols: Vec<crate::ide::SymbolDef> = self
            .send_message_with_reply(message)
            .await
            .with_context(|| format!("failed to resolve symbol '{name}'"))?;

        Ok(symbols)
    }

    async fn find_all_references(
        &mut self,
        symbol: &crate::ide::SymbolDef,
    ) -> anyhow::Result<Vec<crate::ide::FileRange>> {
        if self.test_mode {
            // Return empty result in test mode
            return Ok(vec![]);
        }

        let payload = FindAllReferencesPayload {
            symbol: symbol.clone(),
        };

        let shell_pid = {
            let inner = self.inner.lock().await;
            Some(inner.terminal_shell_pid)
        };

        let message = IPCMessage {
            message_type: IPCMessageType::FindAllReferences,
            id: Uuid::new_v4().to_string(),
            sender: create_message_sender(shell_pid),
            payload: serde_json::to_value(payload)?,
        };

        let locations: Vec<crate::ide::FileRange> = self
            .send_message_with_reply(message)
            .await
            .with_context(|| {
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
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_selection_test_mode() {
        let _ = tracing_subscriber::fmt::try_init();

        let reference_store = Arc::new(crate::reference_store::ReferenceStore::new());
        let ipc = IPCCommunicator::new_test(reference_store);

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
