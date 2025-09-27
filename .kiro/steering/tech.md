# Technology Stack

## Build System & Workspace

- **Cargo Workspace**: Multi-crate Rust workspace with shared dependencies
- **Edition**: Rust 2024 edition
- **Documentation**: mdBook with custom preprocessors and Mermaid diagrams

## Core Technologies

### Backend (MCP Server)
- **Language**: Rust
- **Runtime**: Tokio async runtime
- **MCP SDK**: `rmcp` v0.3.2 for Model Context Protocol implementation
- **Serialization**: Serde with JSON support
- **Error Handling**: `anyhow` and `thiserror`
- **Logging**: `tracing` with structured logging
- **Git Operations**: `git2` for synthetic PR functionality
- **Process Management**: `nix` crate for Unix system calls

### Frontend (VSCode Extension)
- **Language**: TypeScript
- **Build**: Webpack for bundling
- **Runtime**: Node.js 16.x
- **VSCode API**: ^1.74.0
- **Dependencies**: markdown-it, mermaid, shell-quote

### Desktop App (macOS)
- **Language**: Swift
- **Platform**: macOS 14+ (SwiftUI)
- **Package Manager**: Swift Package Manager

## Common Commands

### Building
```bash
# Build entire workspace
cargo build

# Build specific component
cargo build -p symposium-mcp

# Build VSCode extension
cd symposium/vscode-extension && npm run webpack

# Build macOS app
cd symposium/macos-app && swift build
```

### Testing
```bash
# Run all Rust tests
cargo test

# Run specific crate tests
cargo test -p symposium-mcp

# Run integration tests
cargo test --test daemon_integration_tests
```

### Documentation
```bash
# Serve documentation locally
mdbook serve

# Build documentation
mdbook build
```

### Development
```bash
# Run MCP server in debug mode
cargo run -p symposium-mcp -- server

# Debug IPC messages
cargo run -p symposium-mcp -- debug

# Setup development environment
cargo run -p setup
```

## Key Dependencies

- **IPC Communication**: Unix sockets for inter-process communication
- **Async**: Full Tokio feature set for async operations
- **CLI**: Clap v4 for command-line interfaces
- **Testing**: tokio-test, expect-test for Rust testing