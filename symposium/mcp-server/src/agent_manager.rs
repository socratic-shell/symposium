//! Agent Process Manager
//! 
//! Manages persistent, asynchronous AI agents by wrapping CLI tools (q chat, claude-code)
//! in tmux sessions for background execution and attach/detach capabilities.
//!
//! Future consideration: Replace tmux with custom Rust pty manager using crates:
//! - `tty_spawn` - for spawning processes in pseudo-terminals
//! - `teetty` (https://github.com/mitsuhiko/teetty) - for terminal session management
//! This would give us more control over session lifecycle and eliminate tmux dependency.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, info, warn};

/// Agent session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub uuid: String,
    pub tmux_session_name: String,
    pub agent_command: Vec<String>,
    pub working_directory: PathBuf,
    pub status: AgentStatus,
    pub created_at: SystemTime,
    pub last_attached: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Starting,
    Running,
    Crashed,
    Stopped,
}

/// Manages persistent agent sessions using tmux
pub struct AgentManager {
    sessions: HashMap<String, AgentSession>,
    sessions_file: PathBuf,
}

impl AgentManager {
    /// Create new agent manager with persistent session storage
    pub async fn new(sessions_file: PathBuf) -> Result<Self> {
        let mut manager = Self {
            sessions: HashMap::new(),
            sessions_file,
        };
        
        // Load existing sessions from disk
        manager.load_sessions().await?;
        
        // Sync with actual tmux sessions on startup
        manager.sync_with_tmux().await?;
        
        Ok(manager)
    }

    /// Spawn a new persistent agent session
    pub async fn spawn_agent(
        &mut self,
        uuid: String,
        agent_command: Vec<String>,
        working_directory: PathBuf,
    ) -> Result<()> {
        info!("Spawning agent session {} with command: {:?}", uuid, agent_command);

        // Generate unique tmux session name
        let tmux_session_name = format!("symposium-agent-{}", uuid);

        // Check if session already exists
        if self.sessions.contains_key(&uuid) {
            return Err(anyhow!("Agent session {} already exists", uuid));
        }

        // Create tmux session with agent command
        let mut tmux_cmd = Command::new("tmux");
        tmux_cmd
            .arg("new-session")
            .arg("-d") // detached
            .arg("-s")
            .arg(&tmux_session_name)
            .arg("-c")
            .arg(&working_directory);

        // Add the agent command
        for (i, arg) in agent_command.iter().enumerate() {
            if i == 0 {
                tmux_cmd.arg(arg);
            } else {
                tmux_cmd.arg(arg);
            }
        }

        let output = tmux_cmd.output()
            .with_context(|| format!("Failed to execute tmux command for session {}", uuid))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!(
                "Failed to create tmux session {}:\n  stdout: {}\n  stderr: {}",
                uuid,
                stdout.trim(),
                stderr.trim()
            ));
        }

        // Create session metadata
        let session = AgentSession {
            uuid: uuid.clone(),
            tmux_session_name,
            agent_command,
            working_directory,
            status: AgentStatus::Starting,
            created_at: SystemTime::now(),
            last_attached: None,
        };

        // Store session
        self.sessions.insert(uuid.clone(), session);
        self.save_sessions().await?;

        info!("Agent session {} spawned successfully", uuid);
        Ok(())
    }

    /// Execute attach to an agent session (blocks until detach)
    pub async fn execute_attach(&self, uuid: &str) -> Result<()> {
        let session = self.sessions.get(uuid)
            .ok_or_else(|| anyhow!("Agent session {} not found", uuid))?;

        info!("Attaching to agent session {}", uuid);

        // Execute tmux attach command
        let status = std::process::Command::new("tmux")
            .arg("attach-session")
            .arg("-t")
            .arg(&session.tmux_session_name)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to attach to tmux session {}", session.tmux_session_name));
        }

        info!("Detached from agent session {}", uuid);
        Ok(())
    }

    /// Get connection command for attaching to an agent session
    pub fn get_attach_command(&self, uuid: &str) -> Result<Vec<String>> {
        let session = self.sessions.get(uuid)
            .ok_or_else(|| anyhow!("Agent session {} not found", uuid))?;

        Ok(vec![
            "tmux".to_string(),
            "attach-session".to_string(),
            "-t".to_string(),
            session.tmux_session_name.clone(),
        ])
    }

    /// List all agent sessions
    pub fn list_sessions(&self) -> Vec<&AgentSession> {
        self.sessions.values().collect()
    }

    /// Get specific agent session
    pub fn get_session(&self, uuid: &str) -> Option<&AgentSession> {
        self.sessions.get(uuid)
    }

    /// Kill an agent session
    pub async fn kill_agent(&mut self, uuid: &str) -> Result<()> {
        let session = self.sessions.get(uuid)
            .ok_or_else(|| anyhow!("Agent session {} not found", uuid))?;

        info!("Killing agent session {}", uuid);

        // Kill tmux session
        let output = Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(&session.tmux_session_name)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to kill tmux session: {}", stderr);
        }

        // Remove from our tracking
        self.sessions.remove(uuid);
        self.save_sessions().await?;

        info!("Agent session {} killed", uuid);
        Ok(())
    }

    /// Sync our session tracking with actual tmux sessions
    async fn sync_with_tmux(&mut self) -> Result<()> {
        debug!("Syncing with tmux sessions");

        // Get list of tmux sessions
        let output = Command::new("tmux")
            .arg("list-sessions")
            .arg("-F")
            .arg("#{session_name}")
            .output();

        let tmux_sessions = match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| line.starts_with("symposium-agent-"))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            }
            _ => {
                debug!("No tmux sessions found or tmux not available");
                Vec::new()
            }
        };

        // Update session statuses
        for session in self.sessions.values_mut() {
            if tmux_sessions.contains(&session.tmux_session_name) {
                if matches!(session.status, AgentStatus::Crashed | AgentStatus::Stopped) {
                    session.status = AgentStatus::Running;
                }
            } else {
                session.status = AgentStatus::Crashed;
            }
        }

        // Remove sessions that no longer exist in tmux
        let mut to_remove = Vec::new();
        for (uuid, session) in &self.sessions {
            if !tmux_sessions.contains(&session.tmux_session_name) {
                to_remove.push(uuid.clone());
            }
        }

        for uuid in to_remove {
            warn!("Removing orphaned session {}", uuid);
            self.sessions.remove(&uuid);
        }

        self.save_sessions().await?;
        Ok(())
    }

    /// Load sessions from persistent storage
    async fn load_sessions(&mut self) -> Result<()> {
        if !self.sessions_file.exists() {
            debug!("No existing sessions file found");
            return Ok(());
        }

        let content = fs::read_to_string(&self.sessions_file).await?;
        let sessions: HashMap<String, AgentSession> = serde_json::from_str(&content)?;
        
        self.sessions = sessions;
        info!("Loaded {} agent sessions from disk", self.sessions.len());
        Ok(())
    }

    /// Save sessions to persistent storage
    async fn save_sessions(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.sessions)?;
        
        // Ensure parent directory exists
        if let Some(parent) = self.sessions_file.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(&self.sessions_file, content).await?;
        debug!("Saved {} agent sessions to disk", self.sessions.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn tmux_available() -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn test_agent_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let sessions_file = temp_dir.path().join("sessions.json");
        
        let manager = AgentManager::new(sessions_file).await.unwrap();
        assert_eq!(manager.sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_session_persistence() {
        if !tmux_available() {
            eprintln!("⏭️  Skipping test_session_persistence: tmux not available");
            return;
        }
        let temp_dir = tempdir().unwrap();
        let sessions_file = temp_dir.path().join("sessions.json");
        
        // Create manager and add session
        {
            let mut manager = AgentManager::new(sessions_file.clone()).await.unwrap();
            manager.spawn_agent(
                "test-uuid".to_string(),
                vec!["sleep".to_string(), "30".to_string()],
                temp_dir.path().to_path_buf(),
            ).await.unwrap();
            
            // Verify session was created
            assert_eq!(manager.sessions.len(), 1);
            assert!(manager.sessions.contains_key("test-uuid"));
        }
        
        // Kill the tmux session to simulate it dying
        let _ = Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg("symposium-agent-test-uuid")
            .output();
        
        // Create new manager and verify session was loaded but then cleaned up during sync
        {
            let manager = AgentManager::new(sessions_file).await.unwrap();
            // After sync_with_tmux, the dead session should be removed
            assert_eq!(manager.sessions.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_session_file_persistence() {
        let temp_dir = tempdir().unwrap();
        let sessions_file = temp_dir.path().join("sessions.json");
        
        // Create manager and manually add session (without tmux)
        {
            let mut manager = AgentManager {
                sessions: HashMap::new(),
                sessions_file: sessions_file.clone(),
            };
            
            let session = AgentSession {
                uuid: "test-uuid".to_string(),
                tmux_session_name: "symposium-agent-test-uuid".to_string(),
                agent_command: vec!["sleep".to_string(), "30".to_string()],
                working_directory: temp_dir.path().to_path_buf(),
                status: AgentStatus::Running,
                created_at: SystemTime::now(),
                last_attached: None,
            };
            
            manager.sessions.insert("test-uuid".to_string(), session);
            manager.save_sessions().await.unwrap();
        }
        
        // Create new manager and verify session was loaded from file
        {
            let mut manager = AgentManager {
                sessions: HashMap::new(),
                sessions_file: sessions_file.clone(),
            };
            manager.load_sessions().await.unwrap();
            
            assert_eq!(manager.sessions.len(), 1);
            assert!(manager.sessions.contains_key("test-uuid"));
        }
    }
}
