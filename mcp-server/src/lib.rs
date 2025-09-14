//! Dialectic MCP Server Library
//!
//! Rust implementation of the Dialectic MCP server for code review integration.

pub mod constants;
mod daemon;
mod dialect;
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

pub use daemon::{run_daemon_with_idle_timeout, run_client};
pub use pid_discovery::find_vscode_pid_from_mcp;
pub use reference_store::ReferenceStore;
pub use server::DialecticServer;
