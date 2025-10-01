#!/usr/bin/env cargo run --

//! Symposium MCP Server - Rust Implementation
//!
//! Provides tools for AI assistants to display code reviews in VSCode.
//! Acts as a communication bridge between AI and the VSCode extension via IPC.

use anyhow::{Result, Context};
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info, warn};
use std::collections::HashMap;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::path::Path;

use symposium_mcp::{
    AgentManager,
    SymposiumServer,
    constants::DAEMON_SOCKET_PREFIX,
    structured_logging,
};

// git MCP Configuration Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpServerConfig {
    name: String,
    description: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    transport: TransportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransportConfig {
    #[serde(rename = "type")]
    transport_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpConfig {
    servers: HashMap<String, McpServerConfig>,
}

struct McpServerProcess {
    config: McpServerConfig,
    process: Option<Child>,
}

struct McpManager {
    servers: Arc<Mutex<HashMap<String, McpServerProcess>>>,
    config: McpConfig,
}

impl McpManager {
    fn new(config: McpConfig) -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
    async fn start_server(&self, server_id: &str) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if servers.contains_key(server_id) { warn!("MCP server '{}' is already running", server_id);return Ok(());}

        let config = self.config.servers.get(server_id)
            .context(format!("MCP server '{}' not found in configuration", server_id))?;

        info!("Starting MCP server: {}", config.name);
        let mut cmd = ProcessCommand::new(&config.command);
        cmd.args(&config.args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        for (key, value) in &config.env {
            let expanded_value = expand_env_var(value);
            cmd.env(key, expanded_value);
        }
        let process = cmd.spawn()
            .context(format!("Failed to start MCP server '{}'", server_id))?;
        let server_process = McpServerProcess {
            config: config.clone(),
            process: Some(process),
        };
        servers.insert(server_id.to_string(), server_process);
        info!("MCP server '{}' started successfully", server_id);
        Ok(())
    }
    async fn stop_server(&self, server_id: &str) -> Result<()> {
        let mut servers = self.servers.lock().await;
        
        if let Some(mut server_process) = servers.remove(server_id) {
            if let Some(mut process) = server_process.process.take() {
                info!("Stopping MCP server: {}", server_id);
                match process.kill() {
                    Ok(_) => {
                        let _ = process.wait();
                        info!("MCP server '{}' stopped successfully", server_id);
                    }
                    Err(e) => {
                        error!("Failed to stop MCP server '{}': {}", server_id, e);
                        return Err(e.into());
                    }
                }
            }
        } else {
            warn!("MCP server '{}' is not running", server_id);
        }
        Ok(())
    }

    async fn start_all_servers(&self) -> Result<()> {
        info!("Starting all configured MCP servers");
        for server_id in self.config.servers.keys() {
            if let Err(e) = self.start_server(server_id).await {
                error!("Failed to start MCP server '{}': {}", server_id, e);
            }
        }
        Ok(())
    }

    async fn stop_all_servers(&self) -> Result<()> {
        info!("Stopping all MCP servers");
        let server_ids: Vec<String> = {
            let servers = self.servers.lock().await;
            servers.keys().cloned().collect()
        };
        for server_id in server_ids {
            if let Err(e) = self.stop_server(&server_id).await {
                error!("Failed to stop MCP server '{}': {}", server_id, e);
            }
        }
        Ok(())
    }
}

fn expand_env_var(value: &str) -> String {
    if value.starts_with("${") && value.ends_with("}") {
        let var_name = &value[2..value.len()-1];
        std::env::var(var_name).unwrap_or_else(|_| {
            warn!("Environment variable '{}' not found, using empty string", var_name);
            String::new()
        })
    } else {
        value.to_string()
    }
}

fn load_mcp_config<P: AsRef<Path>>(config_path: P) -> Result<McpConfig> {
    let config_content = std::fs::read_to_string(&config_path)
        .context(format!("Failed to read MCP config file: {:?}", config_path.as_ref()))?;
    
    let config: McpConfig = serde_json::from_str(&config_content)
        .context("Failed to parse MCP configuration JSON")?;
    
    Ok(config)
}

fn get_default_config_path() -> std::path::PathBuf {
    let possible_paths = [
        "mcp-servers.json",
        "config/mcp-servers.json", 
        "../mcp-servers.json",
        "../../mcp-servers.json",
    ];
    
    for path in &possible_paths {
        if Path::new(path).exists() {
            return Path::new(path).to_path_buf();
        }
    }
    
    Path::new("mcp-servers.json").to_path_buf()
}

#[derive(Parser)]
#[command(name = "symposium-mcp")]
#[command(about = "Symposium MCP Server for VSCode integration")]
struct Args {
    #[command(flatten)]
    options: Options,

    #[command(subcommand)]
    command: Option<Command>,
}

use symposium_mcp::Options;

#[derive(Parser, Debug)]
struct DaemonArgs {
    /// Optional filename prefix to use (for testing)
    #[arg(long)]
    prefix: Option<String>,

    /// Identity prefix for debug logging
    #[arg(long, default_value = "client")]
    identity_prefix: String,
}

#[derive(Parser, Debug)]
enum Command {
    /// Run PID discovery probe and exit (for testing)
    Probe {},

    /// Run as message bus daemon for multi-window support
    Daemon {
        #[command(flatten)]
        daemon_args: DaemonArgs,

        /// Idle timeout in seconds before auto-shutdown (default: 30)
        #[arg(long, default_value = "30")]
        idle_timeout: u64,
    },

    /// Run as client - connects to daemon and bridges stdin/stdout
    Client {
        #[command(flatten)]
        daemon_args: DaemonArgs,

        /// Auto-start daemon if not running
        #[arg(long, default_value = "true")]
        auto_start: bool,
    },

    /// Debug daemon functionality
    #[command(subcommand)]
    Debug(DebugCommand),

    /// Manage persistent agent sessions
    #[command(subcommand)]
    Agent(AgentCommand),

    /// Manage MCP servers
    #[command(subcommand)]
    Mcp(McpCommand),
}

#[derive(Parser, Debug)]
enum DebugCommand {
    /// Dump recent daemon messages
    DumpMessages {
        #[command(flatten)]
        daemon_args: DaemonArgs,

        /// Number of recent messages to show
        #[arg(long, default_value = "50")]
        count: usize,

        /// Output as JSON instead of human-readable format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Parser, Debug)]
enum AgentCommand {
    /// Spawn a new persistent agent session
    Spawn {
        /// Unique identifier for the agent session
        #[arg(long)]
        uuid: String,

        /// Working directory for the agent
        #[arg(long)]
        workdir: String,

        /// Agent command to run (e.g., "q chat --resume")
        agent_args: Vec<String>,
    },

    /// List all agent sessions
    List,

    /// Get attach command for an agent session
    Attach {
        /// Agent session UUID
        uuid: String,
    },

    /// Kill an agent session
    Kill {
        /// Agent session UUID
        uuid: String,
    },
}

#[derive(Parser, Debug)]
enum McpCommand {
    /// Start an MCP server
    Start {
        /// Server ID to start
        server_id: String,
    },

    /// Stop an MCP server
    Stop {
        /// Server ID to stop
        server_id: String,
    },

    /// List all configured MCP servers
    List,

    /// Start all configured MCP servers
    StartAll,

    /// Stop all running MCP servers
    StopAll,

    /// Show status of MCP servers
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize structured logging with component-specific prefixes
    let flush_guard = structured_logging::init_component_tracing(args.options.dev_log)
        .expect("Failed to initialize logging");

    info!("🔍 PROBE MODE DETECTED - Running PID discovery probe...");

    match args.command {
        Some(Command::Probe {}) => {
            info!("🔍 PROBE MODE DETECTED - Running PID discovery probe...");
            run_pid_probe().await?;
            info!("🔍 PROBE MODE COMPLETE - Exiting");
        }
        Some(Command::Daemon {
            daemon_args,
            idle_timeout,
        }) => {
            let prefix = match &daemon_args.prefix {
                Some(s) => s,
                None => DAEMON_SOCKET_PREFIX,
            };
            info!(
                "🚀 DAEMON MODE - Starting message bus daemon with prefix {prefix}, idle timeout {idle_timeout}s",
            );
            symposium_mcp::run_daemon_with_idle_timeout(prefix, idle_timeout, None).await?;
        }
        Some(Command::Client { daemon_args, auto_start }) => {
            let prefix = match &daemon_args.prefix {
                Some(s) => s,
                None => DAEMON_SOCKET_PREFIX,
            };
            info!("🔌 CLIENT MODE - Connecting to daemon with prefix {prefix}",);
            symposium_mcp::run_client(prefix, auto_start, &daemon_args.identity_prefix, args.options.clone()).await?;
        }
        Some(Command::Debug(debug_cmd)) => {
            run_debug_command(debug_cmd).await?;
        }
        Some(Command::Agent(agent_cmd)) => {
            info!("🤖 AGENT MANAGER MODE");
            run_agent_manager(agent_cmd).await?;
        }
        Some(Command::Mcp(mcp_cmd)) => {
            info!("MCP MANAGER MODE");
            run_mcp_manager(mcp_cmd).await?;
        }
        None => {
            info!("Starting Symposium MCP Server (Rust)");
            info!("MCP Server working directory: {:?}", std::env::current_dir());

            // Create our server instance
            let server = SymposiumServer::new(args.options.clone()).await?;

            // Clone the IPC communicator for shutdown handling
            let ipc_for_shutdown = server.ipc().clone();

            // Start the MCP server with stdio transport
            let service = server.serve(stdio()).await.inspect_err(|e| {
                error!("MCP server error: {:?}", e);
            })?;

            info!("Symposium MCP Server is ready and listening");

            // Wait for the service to complete
            service.waiting().await?;

            info!("Symposium MCP Server shutting down");

            // Send Goodbye discovery message before shutdown
            if let Err(e) = ipc_for_shutdown.shutdown().await {
                error!("Error during IPC shutdown: {}", e);
            }
        }
    }
    std::mem::drop(flush_guard);
    Ok(())
}

/// Run PID discovery probe for testing
async fn run_pid_probe() -> Result<()> {
    use std::process;
    use tracing::{error, info};

    info!("=== SYMPOSIUM MCP SERVER PID PROBE ===");

    let current_pid = process::id();
    info!("MCP Server PID: {}", current_pid);

    // Try to walk up the process tree to find VSCode
    match symposium_mcp::find_vscode_pid_from_mcp(current_pid).await {
        Ok(Some((vscode_pid, terminal_shell_pid))) => {
            info!("✅ SUCCESS: Found VSCode PID: {}", vscode_pid);
            info!("✅ SUCCESS: Terminal Shell PID: {}", terminal_shell_pid);
            info!("🎯 RESULT: MCP server can connect to VSCode via PID-based discovery");
        }
        Ok(None) => {
            error!("❌ FAILED: Could not find VSCode PID in process tree");
            info!("💡 This might mean:");
            info!("   - MCP server not running from VSCode terminal");
            info!("   - Process tree structure is different than expected");
        }
        Err(e) => {
            error!("❌ ERROR: PID discovery failed: {}", e);
        }
    }

    info!("=== END PID PROBE ===");
    Ok(())
}

/// Run agent manager commands
async fn run_agent_manager(agent_cmd: AgentCommand) -> Result<()> {
    use std::path::PathBuf;
    
    // Default sessions file location
    let sessions_file = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
        .join(".symposium")
        .join("agent-sessions.json");

    let mut manager = AgentManager::new(sessions_file).await?;

    match agent_cmd {
        AgentCommand::Spawn { uuid, workdir, agent_args } => {
            let workdir = PathBuf::from(workdir);
            manager.spawn_agent(uuid, agent_args, workdir).await?;
            println!("Agent session spawned successfully");
        }
        AgentCommand::List => {
            let sessions = manager.list_sessions();
            if sessions.is_empty() {
                println!("No active agent sessions");
            } else {
                println!("Active agent sessions:");
                for session in sessions {
                    println!("  {} - {:?} ({})", 
                        session.uuid, 
                        session.status,
                        session.tmux_session_name
                    );
                }
            }
        }
        AgentCommand::Attach { uuid } => {
            manager.execute_attach(&uuid).await?;
        }
        AgentCommand::Kill { uuid } => {
            manager.kill_agent(&uuid).await?;
            println!("Agent session {} killed", uuid);
        }
    }

    Ok(())
}

/// Run MCP manager commands
async fn run_mcp_manager(mcp_cmd: McpCommand) -> Result<()> {
    // Load MCP configuration
    let config_path = get_default_config_path();
    let config = load_mcp_config(&config_path)
        .context(format!("Failed to load MCP config from {:?}", config_path))?;
    
    let manager = McpManager::new(config.clone());
    
    match mcp_cmd {
        McpCommand::Start { server_id } => {
            info!("Starting MCP server: {}", server_id);
            manager.start_server(&server_id).await?;
            println!("✅ MCP server '{}' started successfully", server_id);
        }
        McpCommand::Stop { server_id } => {
            info!("Stopping MCP server: {}", server_id);
            manager.stop_server(&server_id).await?;
            println!("MCP server '{}' stopped successfully", server_id);
        }
        McpCommand::List => {
            println!(" Configured MCP servers:");
            for (server_id, server_config) in &config.servers {
                println!("  • {} - {}", server_id, server_config.description);
            }
        }
        McpCommand::StartAll => {
            info!("Starting all MCP servers");
            manager.start_all_servers().await?;
            println!("✅ All MCP servers started");
        }
        McpCommand::StopAll => {
            info!("Stopping all MCP servers");
            manager.stop_all_servers().await?;
            println!(" All MCP servers stopped");
        }
        McpCommand::Status => {
            println!("📊 MCP Server Status:");
            let running_servers: Vec<String> = {
                let servers = manager.servers.lock().await;
                servers.keys().cloned().collect()
            };
            for (server_id, server_config) in &config.servers {
                let status = if running_servers.contains(server_id) {
                    "🟢 Running"
                } else {
                    "🔴 Stopped"
                };
                println!("  • {} - {} [{}]", server_id, server_config.description, status);
            }
        }
    }
    
    Ok(())
}

async fn run_debug_command(debug_cmd: DebugCommand) -> Result<()> {
    use symposium_mcp::constants;
    use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
    use tokio::net::UnixStream;
    
    match debug_cmd {
        DebugCommand::DumpMessages { daemon_args, count, json } => {
            let socket_prefix = daemon_args.prefix.as_deref().unwrap_or(constants::DAEMON_SOCKET_PREFIX);
            let socket_path = constants::daemon_socket_path(socket_prefix);
            
            // Connect to daemon
            let stream = match UnixStream::connect(&socket_path).await {
                Ok(stream) => stream,
                Err(e) => {
                    println!("Failed to connect to daemon at {}: {}", socket_path, e);
                    println!("Make sure the daemon is running.");
                    return Ok(());
                }
            };
            
            let (reader, mut writer) = stream.into_split();
            
            // Send debug command
            writer.write_all(b"#debug_dump_messages\n").await?;
            writer.flush().await?;
            
            // Read response (single JSON line)
            let mut response = String::new();
            let mut buf_reader = tokio::io::BufReader::new(reader);
            buf_reader.read_line(&mut response).await?;
            
            if response.trim().is_empty() {
                println!("No messages in daemon history.");
                return Ok(());
            }
            
            // Parse JSON response
            let messages: Vec<serde_json::Value> = match serde_json::from_str(response.trim()) {
                Ok(msgs) => msgs,
                Err(e) => {
                    println!("Failed to parse daemon response: {}", e);
                    println!("Raw response: {}", response.trim());
                    return Ok(());
                }
            };
            
            let recent_messages = if messages.len() > count {
                &messages[messages.len() - count..]
            } else {
                &messages
            };
            
            if json {
                // Output as JSON
                println!("{}", serde_json::to_string_pretty(&recent_messages)?);
            } else {
                // Output as human-readable format
                println!("Recent daemon messages ({} of {} total):", recent_messages.len(), messages.len());
                println!("{}", "─".repeat(80));
                
                for msg in recent_messages {
                    if let (Some(timestamp), Some(identifier), Some(content)) = (
                        msg.get("timestamp").and_then(|v| v.as_u64()),
                        msg.get("from_identifier").and_then(|v| v.as_str()),
                        msg.get("content").and_then(|v| v.as_str())
                    ) {
                        let time_str = chrono::DateTime::from_timestamp_millis(timestamp as i64)
                            .unwrap_or_default()
                            .format("%H:%M:%S%.3f");
                        
                        println!("[{}, {}] {}", time_str, identifier, content);
                    } else {
                        println!("Malformed message: {}", msg);
                    }
                }
            }
        }
    }
    
    Ok(())
}
