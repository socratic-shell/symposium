//! Dialectic MCP Server Library
//!
//! Rust implementation of the Dialectic MCP server for code review integration.

mod daemon;
mod dialect;
mod ide;
mod ipc;
mod pid_discovery;
mod reference_store;
mod walkthrough_parser;
mod server;
mod types;
pub mod synthetic_pr;

pub use daemon::{run_daemon_with_prefix, spawn_daemon_process};
pub use pid_discovery::find_vscode_pid_from_mcp;
pub use reference_store::ReferenceStore;
pub use server::DialecticServer;
