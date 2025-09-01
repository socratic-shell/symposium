#!/usr/bin/env cargo run --

//! Symposium MCP Server - Rust Implementation
//!
//! Provides tools for AI assistants to display code reviews in VSCode.
//! Acts as a communication bridge between AI and the VSCode extension via IPC.

use anyhow::Result;
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{Level, error, info};
use tracing_subscriber::{self, EnvFilter};

use symposium_mcp::DialecticServer;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If we are logging to the dev log file
    // then when we drop this flush guard, any final messages
    // will be flushed for sure.
    let mut flush_guard = None;

    // Initialize logging to stderr (MCP uses stdout for protocol).
    // By default, we respect `RUST_LOG` for level etc.
    // In dev mode, we use debug level, and also log to a temporary file.
    if args.dev_log {
        use std::fs::OpenOptions;
        use tracing_appender::non_blocking;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(symposium_mcp::constants::dev_log_path())
            .expect("Failed to open log file");

        let (file_writer, _guard) = non_blocking(file);
        flush_guard = Some(_guard);

        tracing_subscriber::fmt()
            .with_max_level(Level::DEBUG)
            .with_writer(file_writer)
            .with_ansi(false) // No ANSI codes in file
            .init();

        // Also log to stderr for immediate feedback
        eprintln!(
            "Development logging enabled - writing to {} (PID: {})",
            symposium_mcp::constants::dev_log_path(),
            std::process::id()
        );
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .init();
    }

    match args.command {
        Some(Command::Probe {}) => {
            info!("🔍 PROBE MODE DETECTED - Running PID discovery probe...");
            run_pid_probe().await?;
            info!("🔍 PROBE MODE COMPLETE - Exiting");
        }
        Some(Command::Daemon { prefix, idle_timeout }) => {
            let prefix = match &prefix {
                Some(s) => s,
                None => "symposium-daemon",
            };
            info!(
                "🚀 DAEMON MODE - Starting message bus daemon with prefix {prefix}, idle timeout {idle_timeout}s",
            );
            symposium_mcp::run_daemon_with_idle_timeout(prefix, idle_timeout, None).await?;
        }
        Some(Command::Client { prefix, auto_start }) => {
            let prefix = match &prefix {
                Some(s) => s,
                None => "symposium-daemon",
            };
            info!(
                "🔌 CLIENT MODE - Connecting to daemon with prefix {prefix}",
            );
            symposium_mcp::run_client(prefix, auto_start).await?;
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
