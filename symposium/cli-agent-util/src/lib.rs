use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Represents an MCP server configuration
#[derive(Debug, Clone)]
pub struct McpServer {
    pub name: String,
    pub binary_path: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Trait for CLI agents that can have MCP servers installed
pub trait CLIAgent: Send + Sync {
    fn name(&self) -> String;
    fn install_mcp(&self, mcp: &McpServer) -> Result<bool>;
}

/// Q CLI agent implementation
pub struct QCLI {
    executable_path: PathBuf,
}

impl QCLI {
    pub fn detect() -> Result<Box<Self>> {
        let executable_path = which::which("q").context("Q CLI not found in PATH")?;
        Ok(Box::new(QCLI { executable_path }))
    }
}

impl CLIAgent for QCLI {
    fn name(&self) -> String {
        "Q CLI".to_string()
    }

    fn install_mcp(&self, mcp: &McpServer) -> Result<bool> {
        let mut cmd = Command::new(&self.executable_path);

        cmd.args([
            "mcp",
            "add",
            "--name",
            &mcp.name,
            "--command",
            &mcp.binary_path.to_string_lossy(),
            "--force", // Always overwrite existing configuration
        ]);

        // Add arguments
        for arg in &mcp.args {
            cmd.args(["--args", arg]);
        }

        // Add environment variables
        for (key, value) in &mcp.env {
            cmd.args(["--env", &format!("{}={}", key, value)]);
        }

        let output = cmd.output().context("Failed to execute q mcp add")?;

        if output.status.success() {
            println!("✅ MCP server '{}' registered successfully with Q CLI!", mcp.name);
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ Failed to register MCP server with Q CLI:");
            println!("   Error: {}", stderr.trim());
            Ok(false)
        }
    }
}

/// Claude Code agent implementation
pub struct ClaudeCode {
    executable_path: PathBuf,
}

impl ClaudeCode {
    pub fn detect() -> Result<Box<Self>> {
        let executable_path = which::which("claude").context("Claude Code not found in PATH")?;
        Ok(Box::new(ClaudeCode { executable_path }))
    }

    fn is_available(&self) -> bool {
        Command::new(&self.executable_path)
            .args(["mcp", "list"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl CLIAgent for ClaudeCode {
    fn name(&self) -> String {
        "Claude Code".to_string()
    }

    fn install_mcp(&self, mcp: &McpServer) -> Result<bool> {
        if !self.is_available() {
            println!("❌ Claude Code MCP functionality not available");
            return Ok(false);
        }

        // Check if server already exists with correct path
        let list_output = Command::new(&self.executable_path)
            .args(["mcp", "list"])
            .output()
            .context("Failed to execute claude mcp list")?;

        if list_output.status.success() {
            let list_stdout = String::from_utf8_lossy(&list_output.stdout);
            let desired_binary_str = mcp.binary_path.to_string_lossy();

            for line in list_stdout.lines() {
                if line.contains(&mcp.name) && line.contains(desired_binary_str.as_ref()) {
                    println!("✅ MCP server '{}' already configured with correct path", mcp.name);
                    return Ok(true);
                }
            }

            // Remove existing if it exists with wrong path
            if list_stdout.contains(&mcp.name) {
                let _ = Command::new(&self.executable_path)
                    .args(["mcp", "remove", &mcp.name])
                    .output();
            }
        }

        // Add the server
        let mut cmd = Command::new(&self.executable_path);
        cmd.args([
            "mcp",
            "add",
            &mcp.name,
            &mcp.binary_path.to_string_lossy(),
        ]);

        // Add arguments
        for arg in &mcp.args {
            cmd.arg(arg);
        }

        // Add environment variables
        for (key, value) in &mcp.env {
            cmd.args(["-e", &format!("{}={}", key, value)]);
        }

        let output = cmd.output().context("Failed to execute claude mcp add")?;

        if output.status.success() {
            println!("✅ MCP server '{}' registered successfully with Claude Code!", mcp.name);
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("❌ Failed to register MCP server with Claude Code:");
            println!("   Error: {}", stderr.trim());
            Ok(false)
        }
    }
}

/// Detect all available CLI agents
pub fn detect_cli_agents() -> Vec<Box<dyn CLIAgent>> {
    let mut result: Vec<Box<dyn CLIAgent>> = vec![];

    if let Ok(agent) = QCLI::detect() {
        result.push(agent);
    }

    if let Ok(agent) = ClaudeCode::detect() {
        result.push(agent);
    }

    result
}
