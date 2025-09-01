#!/usr/bin/env cargo
//! Symposium Development Setup Tool
//!
//! Builds the Rust MCP server, VSCode extension, and configures them for use
//! with AI assistants like Claude CLI and Q CLI.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, ValueEnum)]
enum CLITool {
    #[value(name = "q")]
    QCli,
    #[value(name = "claude")]
    ClaudeCode,
    #[value(name = "both")]
    Both,
    #[value(name = "auto")]
    Auto,
}

#[derive(Debug, Clone, ValueEnum)]
enum ClaudeScope {
    #[value(name = "user")]
    User,
    #[value(name = "local")]
    Local,
    #[value(name = "project")]
    Project,
}

#[derive(Parser)]
#[command(
    name = "setup",
    about = "Build Symposium components and set up for development with AI assistants",
    long_about = r#"
Build Symposium components and set up for development with AI assistants

This tool builds both the Rust MCP server and VSCode extension, then configures
them for use with Claude CLI or Q CLI.

Examples:
  cargo setup                           # Install to PATH and setup for production use
  cargo setup --dev                     # Build in target/ for development
  cargo setup --tool q                  # Setup for Q CLI only
  cargo setup --tool claude             # Setup for Claude Code only
  cargo setup --tool both               # Setup for both tools

Prerequisites:
  - Rust and Cargo (https://rustup.rs/)
  - Node.js and npm (for VSCode extension)
  - VSCode with 'code' command available
  - Q CLI or Claude Code
"#
)]
struct Args {
    /// Which CLI tool to configure
    #[arg(long, default_value = "auto")]
    tool: CLITool,

    /// Scope for Claude Code MCP configuration
    #[arg(long, default_value = "user")]
    claude_scope: ClaudeScope,

    /// Skip MCP server registration
    #[arg(long)]
    skip_mcp: bool,

    /// Skip VSCode extension build and install
    #[arg(long)]
    skip_extension: bool,

    /// Use development mode (build in target/ directory instead of installing to PATH)
    #[arg(long)]
    dev: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("🎭 Symposium Development Setup");
    println!("{}", "=".repeat(35));

    // Determine which tool to use
    let tool = match args.tool {
        CLITool::Auto => detect_available_tools()?,
        other => other,
    };

    // Check prerequisites
    check_rust()?;
    check_node()?;
    check_vscode()?;

    if !args.skip_mcp {
        match tool {
            CLITool::QCli => check_q_cli()?,
            CLITool::ClaudeCode => check_claude_code()?,
            CLITool::Both => {
                check_q_cli()?;
                check_claude_code()?;
            }
            CLITool::Auto => unreachable!("Auto should have been resolved earlier"),
        }
    }

    // Clean up any existing daemon (for clean dev environment)
    if args.dev {
        cleanup_existing_daemon()?;
    }
    
    // Build components
    let binary_path = if args.dev {
        build_rust_server()?
    } else {
        install_rust_server()?
    };

    if !args.skip_extension {
        build_and_install_extension(args.dev)?;
        
        // Automatically reload VSCode window if in dev mode
        if args.dev {
            reload_vscode_window()?;
        }
    }

    // Setup MCP server(s)
    let mut success = true;
    if !args.skip_mcp {
        match tool {
            CLITool::QCli => {
                success = setup_q_cli_mcp(&binary_path, args.dev)?;
            }
            CLITool::ClaudeCode => {
                success = setup_claude_code_mcp(&binary_path, &args.claude_scope, args.dev)?;
            }
            CLITool::Both => {
                success = setup_q_cli_mcp(&binary_path, args.dev)?
                    && setup_claude_code_mcp(&binary_path, &args.claude_scope, args.dev)?;
            }
            CLITool::Auto => unreachable!("Auto should have been resolved earlier"),
        }
    } else {
        println!("⏭️  Skipping MCP server registration");
    }

    if success {
        print_next_steps(&tool, args.dev)?;
    } else {
        println!("\n❌ Setup incomplete. Please fix the errors above and try again.");
        std::process::exit(1);
    }

    Ok(())
}

fn check_rust() -> Result<()> {
    if which::which("cargo").is_err() {
        return Err(anyhow!(
            "❌ Error: Cargo not found. Please install Rust first.\n   Visit: https://rustup.rs/"
        ));
    }
    Ok(())
}

fn check_node() -> Result<()> {
    if which::which("npm").is_err() {
        return Err(anyhow!(
            "❌ Error: npm not found. Please install Node.js first.\n   Visit: https://nodejs.org/"
        ));
    }
    Ok(())
}

fn check_vscode() -> Result<()> {
    if which::which("code").is_err() {
        return Err(anyhow!(
            "❌ Error: VSCode 'code' command not found. Please install VSCode and ensure the 'code' command is available.\n   Visit: https://code.visualstudio.com/"
        ));
    }
    Ok(())
}

fn check_q_cli() -> Result<()> {
    if which::which("q").is_err() {
        return Err(anyhow!(
            "❌ Error: Q CLI not found. Please install Q CLI first.\n   Visit: https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/q-cli.html"
        ));
    }
    Ok(())
}

fn check_claude_code() -> Result<()> {
    if !is_claude_available() {
        return Err(anyhow!(
            "❌ Error: Claude Code not found. Please install Claude Code first.\n   Visit: https://claude.ai/code"
        ));
    }
    Ok(())
}

fn is_claude_available() -> bool {
    // Check both binary and config directory since claude might be an alias
    which::which("claude").is_ok()
        || home::home_dir().map_or(false, |home| home.join(".claude").exists())
}

fn detect_available_tools() -> Result<CLITool> {
    let has_q = which::which("q").is_ok();
    let has_claude = is_claude_available();

    match (has_q, has_claude) {
        (true, true) => Ok(CLITool::Both),
        (true, false) => Ok(CLITool::QCli),
        (false, true) => Ok(CLITool::ClaudeCode),
        (false, false) => Err(anyhow!(
            "❌ No supported CLI tools found. Please install Q CLI or Claude Code.\n   Q CLI: https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/q-cli.html\n   Claude Code: https://claude.ai/code"
        )),
    }
}

fn get_repo_root() -> Result<PathBuf> {
    // Require CARGO_MANIFEST_DIR - only available when running via cargo
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").context(
        "❌ Setup tool must be run via cargo (e.g., 'cargo setup'). CARGO_MANIFEST_DIR not found.",
    )?;

    let manifest_path = PathBuf::from(manifest_dir);
    // If we're in a workspace member (like setup/), go up to workspace root
    if manifest_path.file_name() == Some(std::ffi::OsStr::new("setup")) {
        if let Some(parent) = manifest_path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    Ok(manifest_path)
}

fn install_rust_server() -> Result<PathBuf> {
    let repo_root = get_repo_root()?;
    let server_dir = repo_root.join("mcp-server");

    println!("📦 Installing Rust MCP server to PATH...");
    println!("   Installing from: {}", server_dir.display());

    // Install the Rust server to ~/.cargo/bin
    let output = Command::new("cargo")
        .args(["install", "--path", ".", "--force"])
        .current_dir(&server_dir)
        .output()
        .context("Failed to execute cargo install")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to install Rust server:\n   Error: {}",
            stderr.trim()
        ));
    }

    // The binary should now be available as 'symposium-mcp' in PATH
    let binary_name = "symposium-mcp";

    // Verify the binary is accessible
    if which::which(binary_name).is_err() {
        println!("⚠️  Warning: symposium-mcp not found in PATH after installation");

        // Try to give helpful guidance about PATH
        if let Some(home) = home::home_dir() {
            let cargo_bin = home.join(".cargo").join("bin");
            println!(
                "   Make sure {} is in your PATH environment variable",
                cargo_bin.display()
            );

            // Check if ~/.cargo/bin exists but isn't in PATH
            if cargo_bin.exists() {
                println!("   Add this to your shell profile (.bashrc, .zshrc, etc.):");
                println!("   export PATH=\"$HOME/.cargo/bin:$PATH\"");
            }
        } else {
            println!("   Make sure ~/.cargo/bin is in your PATH environment variable");
        }
    }

    println!("✅ Rust server installed successfully!");
    Ok(PathBuf::from(binary_name))
}

fn build_rust_server() -> Result<PathBuf> {
    let repo_root = get_repo_root()?;
    let server_dir = repo_root.join("mcp-server");

    println!("🔨 Building Rust MCP server for development...");
    println!("   Building in: {}", server_dir.display());

    // Build the Rust server
    let output = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&server_dir)
        .output()
        .context("Failed to execute cargo build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to build Rust server:\n   Error: {}",
            stderr.trim()
        ));
    }

    // Use CARGO_TARGET_DIR if set, otherwise use workspace default
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| repo_root.join("target").to_string_lossy().to_string());

    // Verify the binary exists
    let binary_path = PathBuf::from(target_dir)
        .join("release")
        .join("symposium-mcp");
    if !binary_path.exists() {
        return Err(anyhow!(
            "❌ Build verification failed: Built binary not found at {}",
            binary_path.display()
        ));
    }

    println!("✅ Rust server built successfully!");
    Ok(binary_path)
}

fn build_and_install_extension(dev_mode: bool) -> Result<()> {
    let repo_root = get_repo_root()?;
    let extension_dir = repo_root.join("ide/vscode");

    println!(
        "\n📦 Building VSCode extension{}...",
        if dev_mode { " (development mode)" } else { "" }
    );

    // Install dependencies
    println!("📥 Installing extension dependencies...");
    let output = Command::new("npm")
        .args(["install"])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute npm install")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to install extension dependencies:\n   Error: {}",
            stderr.trim()
        ));
    }

    // Build extension
    let build_command = if dev_mode { "webpack-dev" } else { "webpack" };
    println!("🔨 Building extension with {}...", build_command);
    let output = Command::new("npm")
        .args(["run", build_command])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute npm run build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to build extension:\n   Error: {}",
            stderr.trim()
        ));
    }

    // Package extension
    println!("📦 Packaging VSCode extension...");
    let output = Command::new("npx")
        .args(["vsce", "package", "--no-dependencies"])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute vsce package")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to package extension:\n   Error: {}",
            stderr.trim()
        ));
    }

    // Find the generated .vsix file
    let entries =
        std::fs::read_dir(&extension_dir).context("Failed to read extension directory")?;

    let mut vsix_file = None;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if let Some(extension) = path.extension() {
            if extension == "vsix" {
                vsix_file = Some(path.file_name().unwrap().to_string_lossy().to_string());
                break;
            }
        }
    }

    let vsix_file = vsix_file.ok_or_else(|| anyhow!("No .vsix file generated"))?;

    // Install extension
    println!(
        "📥 Installing VSCode extension{}...",
        if dev_mode { " (dev build)" } else { "" }
    );
    let output = Command::new("code")
        .args(["--install-extension", &vsix_file])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute code --install-extension")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to install VSCode extension:\n   Error: {}",
            stderr.trim()
        ));
    }

    println!("✅ VSCode extension installed successfully!");
    Ok(())
}

fn setup_q_cli_mcp(binary_path: &Path, dev_mode: bool) -> Result<bool> {
    let mut cmd = Command::new("q");

    if dev_mode {
        // In dev mode, register with --dev-log argument and debug logging
        cmd.args([
            "mcp",
            "add",
            "--name",
            "symposium",
            "--command",
            &binary_path.to_string_lossy(),
            "--args",
            "--dev-log",
            "--env",
            "RUST_LOG=symposium_mcp=debug",
            "--force", // Always overwrite existing configuration
        ]);
    } else {
        // In production mode, register without arguments
        cmd.args([
            "mcp",
            "add",
            "--name",
            "symposium",
            "--command",
            &binary_path.to_string_lossy(),
            "--force", // Always overwrite existing configuration
        ]);
    }

    println!("🔧 Registering Symposium MCP server with Q CLI...");
    println!("   Binary path: {}", binary_path.display());
    if dev_mode {
        println!("   Development mode: logging to /tmp/symposium-mcp.log with RUST_LOG=symposium_mcp=debug");
    }

    let output = cmd.output().context("Failed to execute q mcp add")?;

    if output.status.success() {
        println!("✅ MCP server 'symposium' registered successfully with Q CLI!");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("❌ Failed to register MCP server with Q CLI:");
        println!("   Error: {}", stderr.trim());
        Ok(false)
    }
}

fn setup_claude_code_mcp(binary_path: &Path, scope: &ClaudeScope, dev_mode: bool) -> Result<bool> {
    let scope_str = match scope {
        ClaudeScope::User => "user",
        ClaudeScope::Local => "local",
        ClaudeScope::Project => "project",
    };

    println!("🔧 Configuring Symposium MCP server with Claude Code...");
    println!("   Binary path: {}", binary_path.display());
    println!("   Scope: {}", scope_str);
    if dev_mode {
        println!("   Development mode: logging to /tmp/symposium-mcp.log with RUST_LOG=symposium_mcp=debug");
    }

    // First, check if symposium MCP server is already configured
    println!("🔍 Checking existing MCP server configuration...");
    let list_output = Command::new("claude")
        .args(["mcp", "list"])
        .output()
        .context("Failed to execute claude mcp list")?;

    if !list_output.status.success() {
        let stderr = String::from_utf8_lossy(&list_output.stderr);
        println!("❌ Failed to list MCP servers:");
        println!("   Error: {}", stderr.trim());
        return Ok(false);
    }

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let desired_binary_str = binary_path.to_string_lossy();
    let desired_binary_ref = desired_binary_str.as_ref();
    
    // Parse the output to check if symposium exists and with what binary path
    let mut symposium_exists = false;
    let mut symposium_has_correct_path = false;
    
    for line in list_stdout.lines() {
        // Look for lines that mention symposium MCP server
        // The exact format may vary, but we're looking for symposium name and the binary path
        if line.contains("symposium") {
            symposium_exists = true;
            // Check if this line also contains our desired binary path
            if line.contains(desired_binary_ref) {
                symposium_has_correct_path = true;
                break;
            }
        }
    }

    if symposium_exists && symposium_has_correct_path {
        println!("✅ Symposium MCP server already configured with correct path - no action needed");
        return Ok(true);
    }

    if symposium_exists && !symposium_has_correct_path {
        println!("🔄 Symposium MCP server exists but with different path - updating...");
        
        // Remove the existing server
        let remove_output = Command::new("claude")
            .args(["mcp", "remove", "symposium"])
            .output()
            .context("Failed to execute claude mcp remove")?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            println!("❌ Failed to remove existing MCP server:");
            println!("   Error: {}", stderr.trim());
            return Ok(false);
        }
        println!("✅ Removed existing symposium MCP server");
    }

    // Add the MCP server (either first time or after removal)
    let action = if symposium_exists { "Re-adding" } else { "Adding" };
    println!("➕ {} Symposium MCP server...", action);
    
    let mut cmd = Command::new("claude");
    
    if dev_mode {
        // In dev mode, use add-json with --dev-log argument and debug logging
        let config_json = format!(
            r#"{{"command":"{}","args":["--dev-log"],"env":{{"RUST_LOG":"symposium_mcp=debug"}}}}"#,
            desired_binary_ref
        );
        cmd.args([
            "mcp",
            "add-json",
            "--scope",
            scope_str,
            "symposium",
            &config_json,
        ]);
    } else {
        // In production mode, register without arguments
        cmd.args([
            "mcp",
            "add",
            "--scope",
            scope_str,
            "symposium",
            desired_binary_ref,
        ]);
    }
    
    let add_output = cmd.output().context("Failed to execute claude mcp add")?;

    if add_output.status.success() {
        println!("✅ Symposium MCP server registered successfully with Claude Code!");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        println!("❌ Failed to register MCP server with Claude Code:");
        println!("   Error: {}", stderr.trim());
        Ok(false)
    }
}

fn print_next_steps(tool: &CLITool, dev_mode: bool) -> Result<()> {
    if dev_mode {
        println!("\n🎉 Development setup complete! Symposium is ready for development.");
        println!(
            "🔧 Running in development mode - server will use target/release/symposium-mcp"
        );
    } else {
        println!("\n🎉 Production setup complete! Symposium is installed and ready.");
        println!("📦 Server installed to PATH as 'symposium-mcp'");
    }

    println!("📋 VSCode extension installed and ready to use");

    match tool {
        CLITool::QCli | CLITool::Both => {
            println!("\n🧪 Test with Q CLI:");
            println!("   q chat \"Present a review of the changes you just made\"");
        }
        _ => {}
    }

    match tool {
        CLITool::ClaudeCode | CLITool::Both => {
            println!("\n🧪 Test with Claude Code:");
            println!("   claude chat \"Present a review of the changes you just made\"");
        }
        _ => {}
    }

    println!("\n📝 Next steps:");
    println!("1. Restart VSCode to activate the extension");
    println!("2. Ask your AI assistant to present a code review");
    println!("3. Reviews will appear in the Symposium panel in VSCode");

    if dev_mode {
        println!("\n🔧 Development workflow:");
        println!("- Run 'cargo setup --dev' to rebuild and restart cleanly");
        println!("- For server changes: cd mcp-server && cargo build --release");
        println!("- For extension changes: cd ide/vscode && npm run webpack-dev");
        println!("- VSCode window reloading is handled automatically");
    }

    Ok(())
}

/// Clean up existing daemon process and stale socket files
fn cleanup_existing_daemon() -> Result<()> {
    println!("🧹 Cleaning up existing daemon...");
    
    // Try to gracefully kill any running symposium-mcp daemons
    let output = Command::new("pkill")
        .args(["-TERM", "symposium-mcp"])
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            println!("   ✅ Sent SIGTERM to existing daemon");
            // Give it a moment to shut down gracefully
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Ok(_) => {
            // No process found - that's fine
            println!("   ℹ️  No existing daemon found");
        }
        Err(e) => {
            println!("   ⚠️  Could not check for existing daemon: {}", e);
        }
    }
    
    // Clean up any stale socket files
    let socket_path = "/tmp/symposium-daemon.sock";
    if std::path::Path::new(socket_path).exists() {
        if let Err(e) = std::fs::remove_file(socket_path) {
            println!("   ⚠️  Could not remove stale socket: {}", e);
        } else {
            println!("   ✅ Removed stale socket file");
        }
    }
    
    println!("   🎯 Environment ready for fresh daemon");
    Ok(())
}

/// Reload will be handled automatically by daemon shutdown signal
fn reload_vscode_window() -> Result<()> {
    println!("🔄 VSCode window will reload automatically when daemon restarts...");
    println!("   ℹ️  Daemon sends reload signal to extension on shutdown");
    Ok(())
}
