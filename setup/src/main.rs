#!/usr/bin/env cargo
//! Symposium Development Setup Tool
//!
//! Builds the Rust MCP server, VSCode extension, and configures them for use
//! with AI assistants like Claude CLI and Q CLI.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use symposium_cli_agent_util::{detect_cli_agents, McpServer};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "setup",
    about = "Build Symposium components and set up for development with AI assistants",
    long_about = r#"
Build Symposium components and set up for development with AI assistants

Examples:
  cargo setup                          # Show help and usage
  cargo setup --all                    # Build everything and setup for development
  cargo setup --vscode                 # Build/install VSCode extension only
  cargo setup --mcp                    # Build/install MCP server only
  cargo setup --mcp --restart          # Build/install MCP server and restart daemon
  cargo setup --app                    # Build macOS app only
  cargo setup --app --open             # Build macOS app and launch it

For CI builds, use: cargo ci check / cargo ci test

Prerequisites:
  - Rust and Cargo (https://rustup.rs/)
  - Node.js and npm (for VSCode extension)
  - VSCode with 'code' command available (for development setup)
  - Q CLI or Claude Code (for MCP server setup)
"#
)]
struct Args {
    /// Build all components (VSCode extension, MCP server, Git MCP server, and macOS app)
    #[arg(long)]
    all: bool,

    /// Build/install VSCode extension
    #[arg(long)]
    vscode: bool,

    /// Build/install MCP server
    #[arg(long)]
    mcp: bool,

    /// Build macOS app
    #[arg(long)]
    app: bool,

    /// Open the app after building (requires --app)
    #[arg(long)]
    open: bool,

    /// Restart MCP daemon after building (requires --mcp)
    #[arg(long)]
    restart: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate flag combinations
    if args.open && !args.app && !args.all {
        return Err(anyhow!("❌ --open requires --app"));
    }
    if args.restart && !args.mcp && !args.all {
        return Err(anyhow!("❌ --restart requires --mcp"));
    }

    // Show help if no components specified
    if !args.all && !args.vscode && !args.mcp && !args.app {
        show_help();
        return Ok(());
    }

    // Determine what to build
    let build_vscode = args.all || args.vscode;
    let build_mcp = args.all || args.mcp;
    let build_app = args.all || args.app;

    println!("🐚 Symposium Development Setup");
    println!("{}", "=".repeat(35));

    // Check prerequisites based on what we're building
    check_rust()?;
    if build_vscode {
        check_node()?;
        check_vscode()?;
    }
    if build_mcp {
        check_cli_tools()?;
    }

    // Build components
    let mut binary_path = None;
    if build_mcp {
        binary_path = Some(build_and_install_rust_server()?);
    }
    if build_vscode {
        build_and_install_extension()?;
    }

    if build_app {
        build_macos_app()?;
    }

    // Post-build actions
    if args.restart && build_mcp {
        cleanup_existing_daemon()?;
    }

    if let Some(ref binary_path) = binary_path {
        setup_mcp_servers(binary_path)?;
    }

    print_completion_message(build_vscode, build_mcp, build_app)?;

    if args.open && build_app {
        open_macos_app()?;
    }

    Ok(())
}

fn show_help() {
    println!("🎭 Symposium Development Setup");
    println!("{}", "=".repeat(35));
    println!();
    println!("Common examples:");
    println!("  cargo setup --all --open             # Install everything and open the app");
    println!("  cargo setup --vscode --mcp           # Install VSCode extension and MCP server");
    println!("  cargo setup --mcp                    # Install MCP server");
    println!("  cargo setup --help                   # See all options");
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

fn check_cli_tools() -> Result<()> {
    let agents = detect_cli_agents();
    
    if agents.is_empty() {
        return Err(anyhow!(
            "❌ No supported CLI tools found. Please install Q CLI or Claude Code.\n   Q CLI: https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/q-cli.html\n   Claude Code: https://claude.ai/code"
        ));
    }
    Ok(())
}

fn setup_mcp_servers(binary_path: &Path) -> Result<()> {
    let agents = detect_cli_agents();
    
    if agents.is_empty() {
        println!("⚠️  No CLI agents detected, skipping MCP server setup");
        return Ok(());
    }

    let mcp_server = McpServer {
        name: "symposium".to_string(),
        binary_path: binary_path.to_path_buf(),
        args: vec!["--dev-log".to_string()],
        env: vec![("RUST_LOG".to_string(), "symposium_mcp=debug".to_string())],
    };

    let mut success = true;
    for agent in agents {
        println!("🔧 Registering Symposium MCP server with {}...", agent.name());
        println!("   Binary path: {}", binary_path.display());
        println!("   Development mode: logging to /tmp/symposium-mcp.log with RUST_LOG=symposium_mcp=debug");
        
        match agent.install_mcp(&mcp_server) {
            Ok(result) => success &= result,
            Err(e) => {
                println!("❌ Failed to setup MCP for {}: {}", agent.name(), e);
                success = false;
            }
        }
    }

    if !success {
        println!("\n❌ MCP setup incomplete. Please fix the errors above.");
        std::process::exit(1);
    }

    Ok(())
}

fn print_completion_message(built_vscode: bool, built_mcp: bool, built_app: bool) -> Result<()> {
    println!("\n🎉 Setup complete!");

    if built_mcp {
        println!("📦 MCP server installed to ~/.cargo/bin/symposium-mcp");
    }
    if built_vscode {
        println!("📋 VSCode extension installed and ready to use");
    }
    if built_app {
        println!("🍎 macOS application built and ready to launch");
    }

    if built_mcp {
        println!("\n🧪 Test MCP server:");
        let agents = detect_cli_agents();
        for agent in agents {
            match agent.name().as_str() {
                "Q CLI" => println!("   q chat \"Present a review of the changes you just made\""),
                "Claude Code" => println!("   claude chat \"Present a review of the changes you just made\""),
                _ => {}
            }
        }
    }

    if built_vscode {
        println!("\n📝 Next steps:");
        println!("1. Restart VSCode to activate the extension");
        println!("2. Ask your AI assistant to present a code review");
        println!("3. Reviews will appear in the Symposium panel in VSCode");
    }

    Ok(())
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

fn build_and_install_rust_server() -> Result<PathBuf> {
    let repo_root = get_repo_root()?;
    let server_dir = repo_root.join("symposium/mcp-server");

    println!("📦 Installing Rust MCP server...");
    println!("   Installing from: {}", server_dir.display());

    // Use cargo install --force to always update the binary
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

    println!("✅ Rust server installed successfully!");
    
    // Return full path to the installed binary
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".cargo/bin/symposium-mcp"))
}

fn build_macos_app() -> Result<()> {
    let repo_root = get_repo_root()?;
    let app_dir = repo_root.join("symposium").join("macos-app");

    println!("\n🍎 Building macOS application...");
    println!("   Building in: {}", app_dir.display());

    let output = Command::new("./build-app.sh")
        .current_dir(&app_dir)
        .output()
        .context("Failed to execute build-app.sh")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "❌ Failed to build macOS app:\n   stdout: {}\n   stderr: {}",
            stdout.trim(),
            stderr.trim()
        ));
    }

    println!("✅ macOS application built successfully!");
    Ok(())
}

fn open_macos_app() -> Result<()> {
    let repo_root = get_repo_root()?;
    let app_path = repo_root
        .join("symposium")
        .join("macos-app")
        .join(".build")
        .join("arm64-apple-macosx")
        .join("release")
        .join("Symposium.app");

    println!("\n🚀 Opening Symposium app...");

    let output = Command::new("open")
        .arg(&app_path)
        .output()
        .context("Failed to execute open command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to open app:\n   Error: {}",
            stderr.trim()
        ));
    }

    println!("✅ App launched successfully!");
    Ok(())
}

fn build_and_install_extension() -> Result<()> {
    let repo_root = get_repo_root()?;
    let extension_dir = repo_root.join("symposium/vscode-extension");

    println!("\n📦 Building VSCode extension...");

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
    println!("🔨 Building extension...");
    let output = Command::new("npm")
        .args(["run", "webpack-dev"])
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
    println!("📥 Installing VSCode extension...");
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

/// Clean up existing daemon process and stale socket files
fn cleanup_existing_daemon() -> Result<()> {
    println!("🧹 Cleaning up existing daemon...");

    // Find symposium-mcp daemon processes directly
    let ps_output = Command::new("ps")
        .args(["ux"])
        .output()
        .context("Failed to run ps command")?;

    if !ps_output.status.success() {
        println!("   ⚠️  Could not list processes");
        return Ok(());
    }

    let ps_stdout = String::from_utf8_lossy(&ps_output.stdout);
    let mut killed_any = false;

    for line in ps_stdout.lines() {
        if line.contains("symposium-mcp daemon") {
            // Extract PID (second column in ps ux output)
            if let Some(pid_str) = line.split_whitespace().nth(1) {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    println!("   🎯 Found daemon process: PID {}", pid);

                    // Send SIGTERM to the daemon
                    let kill_result = Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output();

                    match kill_result {
                        Ok(output) if output.status.success() => {
                            println!("   ✅ Sent SIGTERM to daemon PID {}", pid);
                            killed_any = true;
                        }
                        Ok(_) => {
                            println!("   ⚠️  Failed to kill daemon PID {}", pid);
                        }
                        Err(e) => {
                            println!("   ⚠️  Error killing daemon PID {}: {}", pid, e);
                        }
                    }
                }
            }
        }
    }

    if killed_any {
        // Give daemons time to shut down gracefully and send reload signals
        std::thread::sleep(std::time::Duration::from_millis(500));
    } else {
        println!("   ℹ️  No existing daemon processes found");
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
