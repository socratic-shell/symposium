#!/usr/bin/env cargo
//! Socratic Shell Development Setup Tool
//!
//! Builds the Rust MCP server, VSCode extension, and configures them for use
//! with AI assistants like Claude CLI and Q CLI.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "setup",
    about = "Build Socratic Shell components and set up for development with AI assistants",
    long_about = r#"
Build Socratic Shell components and set up for development with AI assistants

Examples:
  cargo setup                           # Show help and usage
  cargo setup --all                    # Build everything and setup for development
  cargo setup --vscode                 # Build/install VSCode extension only
  cargo setup --mcp                    # Build/install MCP server only
  cargo setup --mcp --restart          # Build/install MCP server and restart daemon
  cargo setup --app                    # Build macOS app only
  cargo setup --app --open             # Build macOS app and launch it
  cargo setup --vscode --mcp --app     # Build all components (same as --all)

Prerequisites:
  - Rust and Cargo (https://rustup.rs/)
  - Node.js and npm (for VSCode extension)
  - VSCode with 'code' command available
  - Q CLI or Claude Code (for MCP server)
"#
)]
struct Args {
    /// Build all components (VSCode extension, MCP server, and macOS app)
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

    // Validate flag combinations first
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

    println!("🐚 Socratic Shell Development Setup");
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
    println!("🎭 Socratic Shell Development Setup");
    println!("{}", "=".repeat(35));
    println!();
    println!("Usage: cargo setup [OPTIONS]");
    println!();
    println!("Options:");
    println!(
        "  --all                Build all components (VSCode extension, MCP server, and macOS app)"
    );
    println!("  --vscode             Build/install VSCode extension");
    println!("  --mcp                Build/install MCP server");
    println!("  --app                Build the Symposium macOS app");
    println!("  --open               Open the app after building (requires --app)");
    println!("  --restart            Restart MCP daemon after building (requires --mcp)");
    println!("  --help               Show this help message");
    println!();
    println!("Examples:");
    println!("  cargo setup --all                    # Build everything");
    println!("  cargo setup --vscode                 # Build VSCode extension only");
    println!("  cargo setup --mcp --restart          # Build MCP server and restart daemon");
    println!("  cargo setup --app --open             # Build and launch macOS app");
    println!("  cargo setup --vscode --mcp --app     # Build all components");
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

fn is_claude_available() -> bool {
    // Check both binary and config directory since claude might be an alias
    which::which("claude").is_ok()
        || home::home_dir().map_or(false, |home| home.join(".claude").exists())
}

fn check_cli_tools() -> Result<()> {
    let has_q = which::which("q").is_ok();
    let has_claude = is_claude_available();

    if !has_q && !has_claude {
        return Err(anyhow!(
            "❌ No supported CLI tools found. Please install Q CLI or Claude Code.\n   Q CLI: https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/q-cli.html\n   Claude Code: https://claude.ai/code"
        ));
    }
    Ok(())
}

fn setup_mcp_servers(binary_path: &Path) -> Result<()> {
    let has_q = which::which("q").is_ok();
    let has_claude = is_claude_available();

    let mut success = true;

    if has_q {
        success &= setup_q_cli_mcp(binary_path)?;
    }

    if has_claude {
        success &= setup_claude_code_mcp(binary_path)?;
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
        println!("📦 MCP server installed to ~/.cargo/bin/socratic-shell-mcp");
    }
    if built_vscode {
        println!("📋 VSCode extension installed and ready to use");
    }
    if built_app {
        println!("🍎 macOS application built and ready to launch");
    }

    if built_mcp {
        println!("\n🧪 Test MCP server:");
        if which::which("q").is_ok() {
            println!("   q chat \"Present a review of the changes you just made\"");
        }
        if is_claude_available() {
            println!("   claude chat \"Present a review of the changes you just made\"");
        }
    }

    if built_vscode {
        println!("\n📝 Next steps:");
        println!("1. Restart VSCode to activate the extension");
        println!("2. Ask your AI assistant to present a code review");
        println!("3. Reviews will appear in the Socratic Shell panel in VSCode");
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
    let server_dir = repo_root.join("socratic-shell/mcp-server");

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
    Ok(PathBuf::from(home).join(".cargo/bin/socratic-shell-mcp"))
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
    let extension_dir = repo_root.join("socratic-shell/vscode-extension");

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

fn setup_q_cli_mcp(binary_path: &Path) -> Result<bool> {
    let mut cmd = Command::new("q");

    // Always use dev-log and debug logging for development setup
    cmd.args([
        "mcp",
        "add",
        "--name",
        "socratic-shell",
        "--command",
        &binary_path.to_string_lossy(),
        "--args",
        "--dev-log",
        "--env",
        "RUST_LOG=socratic_shell_mcp=debug",
        "--force", // Always overwrite existing configuration
    ]);

    println!("🔧 Registering Socratic Shell MCP server with Q CLI...");
    println!("   Binary path: {}", binary_path.display());
    println!("   Development mode: logging to /tmp/socratic-shell-mcp.log with RUST_LOG=socratic_shell_mcp=debug");

    let output = cmd.output().context("Failed to execute q mcp add")?;

    if output.status.success() {
        println!("✅ MCP server 'socratic-shell' registered successfully with Q CLI!");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("❌ Failed to register MCP server with Q CLI:");
        println!("   Error: {}", stderr.trim());
        Ok(false)
    }
}

fn setup_claude_code_mcp(binary_path: &Path) -> Result<bool> {
    println!("🔧 Configuring Socratic Shell MCP server with Claude Code...");
    println!("   Binary path: {}", binary_path.display());
    println!("   Development mode: logging to /tmp/socratic-shell-mcp.log with RUST_LOG=socratic_shell_mcp=debug");
    // Check existing configuration
    let list_output = Command::new("claude")
        .args(["mcp", "list"])
        .output()
        .context("Failed to execute claude mcp list")?;

    if !list_output.status.success() {
        let stderr = String::from_utf8_lossy(&list_output.stderr);
        println!("❌ Failed to list MCP servers: {}", stderr.trim());
        return Ok(false);
    }

    let list_stdout = String::from_utf8_lossy(&list_output.stdout);
    let desired_binary_str = binary_path.to_string_lossy();

    // Check if socratic-shell exists with correct path
    let mut socratic_shell_exists = false;
    let mut socratic_shell_has_correct_path = false;

    for line in list_stdout.lines() {
        if line.contains("socratic-shell") {
            socratic_shell_exists = true;
            if line.contains(desired_binary_str.as_ref()) {
                socratic_shell_has_correct_path = true;
                break;
            }
        }
    }

    if socratic_shell_exists && socratic_shell_has_correct_path {
        println!("✅ Socratic Shell MCP server already configured with correct path");
        return Ok(true);
    }

    if socratic_shell_exists {
        println!("🔄 Updating existing socratic-shell MCP server...");
        let remove_output = Command::new("claude")
            .args(["mcp", "remove", "socratic-shell"])
            .output()
            .context("Failed to execute claude mcp remove")?;

        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            println!("❌ Failed to remove existing MCP server: {}", stderr.trim());
            return Ok(false);
        }
    }

    // Add MCP server
    println!("   Development mode: logging to /tmp/socratic-shell-mcp.log with RUST_LOG=socratic_shell_mcp=debug");
    let config_json = format!(
        r#"{{"command":"{}","args":["--dev-log"],"env":{{"RUST_LOG":"socratic_shell_mcp=debug"}}}}"#,
        binary_path.display()
    );

    let add_output = Command::new("claude")
        .args([
            "mcp",
            "add-json",
            "--scope",
            "user",
            "socratic-shell",
            &config_json,
        ])
        .output()
        .context("Failed to execute claude mcp add")?;

    if add_output.status.success() {
        println!("✅ Socratic Shell MCP server registered successfully with Claude Code!");
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        println!(
            "❌ Failed to register MCP server with Claude Code: {}",
            stderr.trim()
        );
        Ok(false)
    }
}

/// Clean up existing daemon process and stale socket files
fn cleanup_existing_daemon() -> Result<()> {
    println!("🧹 Cleaning up existing daemon...");

    // Find socratic-shell-mcp daemon processes directly
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
        if line.contains("socratic-shell-mcp daemon") {
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
    let socket_path = "/tmp/socratic-shell-daemon.sock";
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
