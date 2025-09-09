#!/usr/bin/env cargo run --

//! Symposium MCP Server - Rust Implementation
//!
//! Provides tools for AI assistants to display code reviews in VSCode.
//! Acts as a communication bridge between AI and the VSCode extension via IPC.

use anyhow::Result;
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

use symposium_mcp::{
    AgentManager,
    DialecticServer,
    constants::DAEMON_SOCKET_PREFIX,
    structured_logging::{self, Component},
};

#[derive(Parser)]
#[command(name = "symposium-mcp")]
#[command(about = "Symposium MCP Server for VSCode integration")]
struct Args {
    /// Enable development logging to the default log file
    #[arg(long, global = true)]
    dev_log: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Parser, Debug)]
enum Command {
    /// Run PID discovery probe and exit (for testing)
    Probe {},

    /// Run as message bus daemon for multi-window support
    Daemon {
        /// Optional filename prefix to use (for testing)
        #[arg(long)]
        prefix: Option<String>,

        /// Idle timeout in seconds before auto-shutdown (default: 30)
        #[arg(long, default_value = "30")]
        idle_timeout: u64,
    },

    /// Run as client - connects to daemon and bridges stdin/stdout
    Client {
        /// Optional socket prefix for testing
        #[arg(long)]
        prefix: Option<String>,

        /// Auto-start daemon if not running
        #[arg(long, default_value = "true")]
        auto_start: bool,
    },

    /// Manage persistent agent sessions
    #[command(subcommand)]
    Agent(AgentCommand),
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Determine component type based on command
    let component = match &args.command {
        Some(Command::Daemon { .. }) => Component::Daemon,
        Some(Command::Client { .. }) => Component::Client,
        _ => Component::McpServer,
    };

    // Initialize structured logging with component-specific prefixes
    let flush_guard = structured_logging::init_component_tracing(component, args.dev_log)
        .expect("Failed to initialize logging");

    match args.command {
        Some(Command::Probe {}) => {
            info!("🔍 PROBE MODE DETECTED - Running PID discovery probe...");
            run_pid_probe().await?;
            info!("🔍 PROBE MODE COMPLETE - Exiting");
        }
        Some(Command::Daemon {
            prefix,
            idle_timeout,
        }) => {
            let prefix = match &prefix {
                Some(s) => s,
                None => DAEMON_SOCKET_PREFIX,
            };
            info!(
                "🚀 DAEMON MODE - Starting message bus daemon with prefix {prefix}, idle timeout {idle_timeout}s",
            );
            symposium_mcp::run_daemon_with_idle_timeout(prefix, idle_timeout, None).await?;
        }
        Some(Command::Client { prefix, auto_start }) => {
            let prefix = match &prefix {
                Some(s) => s,
                None => DAEMON_SOCKET_PREFIX,
            };
            info!("🔌 CLIENT MODE - Connecting to daemon with prefix {prefix}",);
            symposium_mcp::run_client(prefix, auto_start).await?;
        }
        Some(Command::Agent(agent_cmd)) => {
            info!("🤖 AGENT MANAGER MODE");
            run_agent_manager(agent_cmd).await?;
        }
        None => {
            info!("Starting Symposium MCP Server (Rust)");

            // Create our server instance
            let server = DialecticServer::new().await?;

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
            let attach_cmd = manager.get_attach_command(&uuid)?;
            println!("To attach to agent session {}, run:", uuid);
            println!("  {}", attach_cmd.join(" "));
        }
        AgentCommand::Kill { uuid } => {
            manager.kill_agent(&uuid).await?;
            println!("Agent session {} killed", uuid);
        }
    }

    Ok(())
}
