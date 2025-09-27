#!/usr/bin/env cargo
//! Symposium CI Tool
//!
//! Builds and tests all Symposium components for continuous integration

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "ci",
    about = "Symposium CI tool for building and testing all components",
    long_about = r#"
Symposium CI tool for building and testing all components

Examples:
  cargo ci                             # Check compilation (default)
  cargo ci check                       # Check that all components compile
  cargo ci test                        # Run all tests

Components:
  - Rust MCP server (cargo check)
  - TypeScript VSCode extension (npm ci + webpack)
  - Swift macOS app (swift build, macOS only)
"#
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

// ANCHOR: commands
#[derive(Parser)]
enum Commands {
    /// Check that all components compile
    Check,
    /// Run all tests
    Test,
}
// ANCHOR_END: commands

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Check) => run_check(),
        Some(Commands::Test) => run_test(),
        None => run_check(), // Default to check
    }
}

// ANCHOR: run_check
/// Check that all components compile
fn run_check() -> Result<()> {
    println!("🤖 Symposium CI Check");
    println!("{}", "=".repeat(26));

    // Check basic prerequisites
    check_rust()?;
    check_node()?;

    // Check all components compile
    check_rust_server()?;
    build_extension()?;
    
    // Only build macOS app on macOS
    if cfg!(target_os = "macos") {
        build_macos_app()?;
    } else {
        println!("⏭️  Skipping macOS app build (not on macOS)");
    }

    println!("\n✅ All components check passed!");
    Ok(())
}
// ANCHOR_END: run_check

// ANCHOR: run_test
/// Run all tests
fn run_test() -> Result<()> {
    println!("🤖 Symposium CI Test");
    println!("{}", "=".repeat(25));

    // Check basic prerequisites
    check_rust()?;
    check_node()?;

    // Run tests for all components
    run_rust_tests()?;
    run_typescript_tests()?;
    
    // Run Swift tests if they exist (macOS only)
    if cfg!(target_os = "macos") {
        run_swift_tests()?;
    } else {
        println!("⏭️  Skipping Swift tests (not on macOS)");
    }

    println!("\n✅ All tests completed!");
    Ok(())
}
// ANCHOR_END: run_test

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

fn get_repo_root() -> Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").context(
        "❌ CI tool must be run via cargo. CARGO_MANIFEST_DIR not found.",
    )?;

    let manifest_path = PathBuf::from(manifest_dir);
    // If we're in the ci/ directory, go up to workspace root
    if manifest_path.file_name() == Some(std::ffi::OsStr::new("ci")) {
        if let Some(parent) = manifest_path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    Ok(manifest_path)
}

// ANCHOR: check_rust_server
/// Check Rust MCP server compilation
fn check_rust_server() -> Result<()> {
    let repo_root = get_repo_root()?;
    let server_dir = repo_root.join("symposium/mcp-server");

    println!("🦀 Checking Rust MCP server...");
    println!("   Checking in: {}", server_dir.display());

    let output = Command::new("cargo")
        .args(["check", "--release"])
        .current_dir(&server_dir)
        .output()
        .context("Failed to execute cargo check")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to check Rust server:\n   Error: {}",
            stderr.trim()
        ));
    }

    println!("✅ Rust server check passed!");
    Ok(())
}
// ANCHOR_END: check_rust_server

// ANCHOR: build_extension
/// Build VSCode extension
fn build_extension() -> Result<()> {
    let repo_root = get_repo_root()?;
    let extension_dir = repo_root.join("symposium/vscode-extension");

    println!("\n📦 Building VSCode extension...");

    // Install dependencies
    println!("📥 Installing extension dependencies...");
    let output = Command::new("npm")
        .args(["ci"])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute npm ci")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to install extension dependencies:\n   Error: {}",
            stderr.trim()
        ));
    }

    // Build extension for production
    println!("🔨 Building extension...");
    let output = Command::new("npm")
        .args(["run", "webpack"])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute npm run webpack")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ Failed to build extension:\n   Error: {}",
            stderr.trim()
        ));
    }

    println!("✅ VSCode extension built successfully!");
    Ok(())
}
// ANCHOR_END: build_extension

// ANCHOR: build_macos_app
/// Build macOS app
fn build_macos_app() -> Result<()> {
    let repo_root = get_repo_root()?;
    let app_dir = repo_root.join("symposium").join("macos-app");

    println!("\n🍎 Building macOS application...");
    println!("   Building in: {}", app_dir.display());

    let output = Command::new("swift")
        .args(["build", "--configuration", "release"])
        .current_dir(&app_dir)
        .output()
        .context("Failed to execute swift build")?;

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
// ANCHOR_END: build_macos_app

/// Run Rust tests
fn run_rust_tests() -> Result<()> {
    let repo_root = get_repo_root()?;

    println!("🦀 Running Rust tests...");
    println!("   Testing workspace in: {}", repo_root.display());

    let status = Command::new("cargo")
        .args(["test", "--workspace"])
        .env("RUST_BACKTRACE", "1")
        .current_dir(&repo_root)
        .status()
        .context("Failed to execute cargo test")?;

    if !status.success() {
        return Err(anyhow!("❌ Rust tests failed"));
    }

    println!("✅ Rust tests passed!");
    Ok(())
}

/// Run TypeScript tests if they exist
fn run_typescript_tests() -> Result<()> {
    let repo_root = get_repo_root()?;
    let extension_dir = repo_root.join("symposium/vscode-extension");

    println!("\n📦 Checking for TypeScript tests...");

    // Check if package.json has a test script
    let package_json_path = extension_dir.join("package.json");
    if !package_json_path.exists() {
        println!("⏭️  No package.json found, skipping TypeScript tests");
        return Ok(());
    }

    let package_json = std::fs::read_to_string(&package_json_path)
        .context("Failed to read package.json")?;

    if !package_json.contains("\"test\"") {
        println!("⏭️  No test script found in package.json, skipping TypeScript tests");
        return Ok(());
    }

    println!("🔨 Running TypeScript tests...");
    let output = Command::new("npm")
        .args(["test"])
        .current_dir(&extension_dir)
        .output()
        .context("Failed to execute npm test")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "❌ TypeScript tests failed:\n   Error: {}",
            stderr.trim()
        ));
    }

    println!("✅ TypeScript tests passed!");
    Ok(())
}

/// Run Swift tests if they exist
fn run_swift_tests() -> Result<()> {
    let repo_root = get_repo_root()?;
    let app_dir = repo_root.join("symposium").join("macos-app");

    println!("\n🍎 Checking for Swift tests...");

    // Check if Tests directory exists
    let tests_dir = app_dir.join("Tests");
    if !tests_dir.exists() {
        println!("⏭️  No Tests directory found, skipping Swift tests");
        return Ok(());
    }

    println!("🔨 Running Swift tests...");
    let output = Command::new("swift")
        .args(["test"])
        .current_dir(&app_dir)
        .output()
        .context("Failed to execute swift test")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "❌ Swift tests failed:\n   stdout: {}\n   stderr: {}",
            stdout.trim(),
            stderr.trim()
        ));
    }

    println!("✅ Swift tests passed!");
    Ok(())
}
