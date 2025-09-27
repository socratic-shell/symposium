# Project Structure

## Root Level Organization

```
symposium/
├── symposium/           # Main application components
│   ├── mcp-server/     # Rust MCP server (core backend)
│   ├── vscode-extension/ # TypeScript VSCode integration
│   └── macos-app/      # Swift desktop application
├── md/                 # Documentation source (mdBook)
├── setup/              # Development environment setup tool
├── md-rfd-preprocessor/ # Custom mdBook preprocessor
└── theme/              # Documentation styling
```

## Core Components

### MCP Server (`symposium/mcp-server/`)
- **`src/actor/`**: Actor system for IPC communication
- **`src/eg/`**: "Example getter" - Rust crate source tools
- **`src/git/`**: Git operations and synthetic PR functionality
- **`src/guidance/`**: Embedded guidance documents for AI agents
- **`src/ide/`**: IDE integration and ambiguity resolution
- **`tests/`**: Integration tests for daemon and tools
- **`test-utils/`**: Shared testing utilities

### VSCode Extension (`symposium/vscode-extension/`)
- **`src/extension.ts`**: Main extension entry point
- **`src/ipc.ts`**: IPC communication with MCP server
- **`src/walkthroughWebview.ts`**: Interactive walkthrough UI
- **`src/reviewProvider.ts`**: Code review integration
- **`out/`**: Compiled JavaScript output

### macOS App (`symposium/macos-app/`)
- **`Sources/Symposium/`**: Swift source code
- **`Sources/Symposium/Models/`**: Data models and managers
- **`Sources/Symposium/Views/`**: SwiftUI view components
- **`Sources/Symposium/Utils/`**: Utility functions

## Documentation (`md/`)

### Key Sections
- **`design/`**: Architecture and implementation details
- **`get-started/`**: User onboarding and tutorials  
- **`ref/`**: API reference and tool documentation
- **`rfds/`**: Request for Discussion documents
- **`research/`**: Technical research and analysis

### Special Files
- **`SUMMARY.md`**: mdBook table of contents
- **`artwork/`**: Logos and visual assets
- **`design/mcp-tools/`**: Detailed MCP tool specifications

## Configuration Files

- **`Cargo.toml`**: Workspace configuration with shared dependencies
- **`book.toml`**: mdBook configuration with preprocessors
- **`.cargo/config.toml`**: Cargo build configuration
- **`features.yaml`**: Feature tracking and status

## Development Patterns

### File Naming
- Rust: `snake_case.rs` for modules, `PascalCase` for types
- TypeScript: `camelCase.ts` for files, `PascalCase` for classes
- Swift: `PascalCase.swift` for files and types
- Documentation: `kebab-case.md` for files

### Module Organization
- **Actor System**: Message-passing architecture in `src/actor/`
- **Tool Categories**: Grouped by functionality (IDE, Git, Rust, etc.)
- **Embedded Resources**: Guidance files compiled into binary
- **Test Structure**: Integration tests mirror source structure

### IPC Architecture
All components communicate through a central message bus implemented in the MCP server, enabling loose coupling between IDE extensions, desktop app, and AI agents.