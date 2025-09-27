# CI Tool ![Implemented](https://img.shields.io/badge/status-implemented-green)

The `cargo ci` tool provides unified build verification and testing for all Symposium components across Rust, TypeScript, and Swift.

## Quick Reference

```bash
cargo ci              # Check compilation (default)
cargo ci check        # Check that all components compile
cargo ci test         # Run all tests
```

## Commands

The CI tool provides two subcommands:

```rust
{{#include ../../ci/src/main.rs:commands}}
```

### Check Command

Verifies that all components compile without running tests:

```rust
{{#include ../../ci/src/main.rs:run_check}}
```

**What it does:**
- Checks Rust MCP server with `cargo check --release`
- Builds TypeScript VSCode extension with `npm ci` + `npm run webpack`
- Builds Swift macOS app with `swift build --configuration release` (macOS only)

### Test Command

Runs test suites for all components:

```rust
{{#include ../../ci/src/main.rs:run_test}}
```

**What it does:**
- Runs Rust tests with `cargo test --workspace`
- Runs TypeScript tests with `npm test` (if test script exists)
- Runs Swift tests with `swift test` (if Tests directory exists, macOS only)

## Component Details

### Rust MCP Server

```rust
{{#include ../../ci/src/main.rs:check_rust_server}}
```

Uses `cargo check --release` for fast type checking without code generation. Runs in `symposium/mcp-server` directory.

### TypeScript VSCode Extension

```rust
{{#include ../../ci/src/main.rs:build_extension}}
```

Two-step process:
1. `npm ci` - Clean install of dependencies from package-lock.json
2. `npm run webpack` - Production build of extension

Runs in `symposium/vscode-extension` directory.

### Swift macOS Application

```rust
{{#include ../../ci/src/main.rs:build_macos_app}}
```

Uses `swift build --configuration release` for production builds. Runs in `symposium/macos-app` directory. Automatically skipped on non-macOS platforms.

## Running Locally

### Check Everything Compiles

```bash
cargo ci check
```

This is the fastest way to verify your changes compile across all platforms.

### Run All Tests

```bash
cargo ci test
```

Runs the full test suite. Note that Swift tests only run on macOS.

### Individual Components

You can also test components directly:

```bash
# Rust only
cd symposium/mcp-server && cargo check --release

# TypeScript only
cd symposium/vscode-extension && npm ci && npm run webpack

# Swift only (macOS)
cd symposium/macos-app && swift build --configuration release
```

## GitHub Actions Integration

The CI tool is used in GitHub Actions workflows with dependency caching:

```yaml
- name: Run CI check
  run: cargo ci check

- name: Run CI tests  
  run: cargo ci test
```

Caching includes:
- Cargo registry and git database
- Cargo target directories
- npm node_modules

See `.github/workflows/ci.yml` for complete configuration.

## Implementation

The CI tool is implemented as a dedicated crate in `ci/` with a cargo alias defined in `.cargo/config.toml`:

```toml
[alias]
ci = "run -p ci --"
```

This allows running `cargo ci` from anywhere in the workspace.