# Installation

*This guide walks you through installing both the VSCode extension and MCP server components.*

## Prerequisites

- **VSCode**: Version 1.74.0 or later
- **Rust**: Latest stable version (for MCP server)
- **Node.js**: Version 18+ (for VSCode extension)
- **AI Assistant**: Compatible with Model Context Protocol (MCP)

## VSCode Extension

### From VSCode Marketplace (Recommended)

1. Open VSCode
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "Dialectic"
4. Click "Install" on the Dialectic extension
5. Reload VSCode when prompted

### From VSIX File

If installing from a local VSIX file:

1. Download the `.vsix` file from the releases page
2. Open VSCode
3. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
4. Click the "..." menu and select "Install from VSIX..."
5. Select the downloaded `.vsix` file
6. Reload VSCode when prompted

## MCP Server

### From Source (Current Method)

The Dialectic MCP server is implemented in Rust for optimal performance and reliability:

```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and install Dialectic
git clone https://github.com/symposium/dialectic.git
cd dialectic
cargo setup
```

The setup tool will:
- Build the Rust MCP server in release mode
- Install the binary globally as `dialectic-mcp-server`
- Build and install the VSCode extension
- Configure your AI assistant (if Claude CLI or Q CLI is detected)

### Setup Options

```bash
# Production setup (installs to PATH)
cargo setup

# Development setup (builds in target/)
cargo setup --dev

# Setup for specific AI assistant
cargo setup --tool claude
cargo setup --tool q
cargo setup --tool both

# Skip extension build (server only)
cargo setup --skip-extension

# Skip MCP registration (build only)
cargo setup --skip-mcp
```

### Manual Installation

If you prefer to install components separately:

```bash
# Build and install the Rust MCP server
cd server
cargo build --release
cargo install --path .

# Build and install the VSCode extension
cd ../extension
npm install
npm run compile
npm run package
code --install-extension dialectic-*.vsix
```

## AI Assistant Configuration

Configure your AI assistant to use the Dialectic MCP server. The exact steps depend on your AI assistant:

### For Amazon Q CLI

Add to your MCP configuration:

```json
{
  "mcpServers": {
    "dialectic": {
      "command": "dialectic-mcp-server",
      "args": []
    }
  }
}
```

### For Other MCP-Compatible Assistants

Refer to your AI assistant's documentation for adding MCP servers. Use:
- **Command**: `dialectic-mcp-server`
- **Transport**: stdio
- **Arguments**: None required

## Verification

### Test VSCode Extension

1. Open VSCode in a project directory
2. Check that "Dialectic" appears in the sidebar (activity bar)
3. The extension should show "Ready" status

### Test MCP Server

1. Start your AI assistant with MCP configuration
2. Verify the `present_review` tool is available:
   ```
   You: "What tools do you have available?"
   AI: "I have access to... present_review (Display a code review in VSCode)..."
   ```

### End-to-End Test

1. Make some code changes in your project
2. Ask your AI assistant: "Present a review of my recent changes"
3. The review should appear in the Dialectic panel in VSCode
4. File references in the review should be clickable

## Troubleshooting

### Extension Not Loading

- Check VSCode version (must be 1.74.0+)
- Reload VSCode window (Ctrl+Shift+P → "Developer: Reload Window")
- Check VSCode Developer Console for errors (Help → Toggle Developer Tools)

### MCP Server Not Found

- Verify installation: `which dialectic-mcp-server`
- Check Rust installation: `cargo --version`
- Try reinstalling: `cargo setup --skip-extension`

### IPC Connection Issues

- Ensure both extension and MCP server are running
- Check that `DIALECTIC_IPC_PATH` environment variable is set in terminal
- Restart VSCode to refresh environment variables

### File References Not Working

- Verify you're in a workspace (not just a single file)
- Check that file paths in reviews are relative to workspace root
- Ensure files exist at the referenced locations

## Configuration

### VSCode Extension Settings

The extension provides these configurable settings:

- **`dialectic.autoShow`**: Automatically show review panel when reviews are received (default: true)
- **`dialectic.maxContentLength`**: Maximum review content length in characters (default: 100000)

Access via File → Preferences → Settings → Search "dialectic"

### MCP Server Options

The MCP server accepts these command-line options:

- **`--log-level`**: Set logging level (debug, info, warn, error) (default: info)
- **`--timeout`**: IPC timeout in milliseconds (default: 5000)

Example:
```bash
dialectic-mcp-server --log-level debug --timeout 10000
```

## Uninstallation

### Remove VSCode Extension

1. Go to Extensions in VSCode
2. Find "Dialectic" extension
3. Click gear icon → "Uninstall"

### Remove MCP Server

```bash
cargo uninstall dialectic-mcp-server
```

### Clean Configuration

Remove any MCP server configuration from your AI assistant's settings.

## Next Steps

- Read the [Quick Start](./quick-start.md) guide for your first review
- Learn about [Review Format](./review-format.md) conventions
- Check [Troubleshooting](./troubleshooting.md) for common issues
