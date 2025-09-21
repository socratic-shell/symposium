//! Dialectic MCP Server Library
//!
//! Rust implementation of the Dialectic MCP server for code review integration.

pub mod actor;
pub mod constants;
mod daemon;
mod dialect;
mod eg;
mod ide;
mod ipc;
mod pid_discovery;
mod reference_store;
pub mod structured_logging;
mod walkthrough_parser;
mod server;
pub mod types;

pub mod git;
mod workspace_dir;
mod agent_manager;

// Re-export Options for use in main.rs
pub use crate::main_types::Options;

mod main_types {
    use clap::Parser;
    use std::process::Command;

    #[derive(Parser, Debug, Clone)]
    pub struct Options {
        /// Enable development logging to the default log file
        #[arg(long, global = true)]
        pub dev_log: bool,
    }

    impl Options {
        /// Reproduce these options on a spawned command
        pub fn reproduce(&self, cmd: &mut Command) {
            // Pass --dev-log if we received it
            if self.dev_log {
                cmd.arg("--dev-log");
            }

            // Pass RUST_LOG environment variable if set
            if let Ok(rust_log) = std::env::var("RUST_LOG") {
                cmd.env("RUST_LOG", rust_log);
            }
        }
    }
}

pub use daemon::{run_daemon_with_idle_timeout, run_client};
pub use pid_discovery::find_vscode_pid_from_mcp;
pub use reference_store::ReferenceStore;
pub use server::SymposiumServer;
pub use agent_manager::AgentManager;
