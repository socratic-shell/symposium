# MCP Server Overview

The Symposium MCP server (`symposium-mcp`) provides a comprehensive set of tools for AI assistants to interact with VSCode and coordinate taskspace orchestration.

## Tool Categories

- **[IDE Integration Tools](./mcp-tools/ide-integration.md)** - Get selections and navigate code structure
- **[Code Walkthrough Tools](./mcp-tools/walkthroughs.md)** - Create interactive code tours and explanations  
- **[Synthetic Pull Request Tools](./mcp-tools/synthetic-prs.md)** - Generate and manage code reviews
- **[Taskspace Orchestration Tools](./mcp-tools/taskspace-orchestration.md)** - Create and coordinate collaborative workspaces
- **[Reference System Tools](./mcp-tools/reference-system.md)** - Manage compact reference storage and retrieval
- **[Rust Development Tools](./mcp-tools/rust-development.md)** - {RFD:rust-crate-sources-tool} Explore Rust crate sources and examples

## Agent Initialization System

The MCP server provides the foundation for dynamic agent initialization through the yiasou prompt system and embedded guidance resources. For complete details, see [Guidance and Initialization](./guidance-and-initialization.md).

Key capabilities:
- `@yiasou` stored prompt with taskspace context
- Embedded guidance resources  like `main.md` and `walkthrough-format.md`.
  - These can fetched using the [`expand_reference` tool from the embedded reference system](./mcp-tools/reference-system.md).

## Architecture

The MCP server operates as a bridge between AI assistants and the VSCode extension:

1. **Process Discovery**: Automatically discovers the parent VSCode process
2. **IPC Communication**: Connects to the daemon message bus via Unix socket
3. **Tool Execution**: Processes MCP tool calls and routes them appropriately
4. **Resource Serving**: Provides embedded guidance files as MCP resources
5. **Dynamic Prompts**: Assembles context-aware initialization prompts
6. **Response Handling**: Returns structured results to the AI assistant

## Configuration

The server is configured through your AI assistant's MCP settings:

```json
{
  "mcpServers": {
    "symposium": {
      "command": "/path/to/symposium-mcp",
      "args": ["server"]
    }
  }
}
```

## Error Handling

All tools include comprehensive error handling:
- **IPC Failures**: Graceful degradation when VSCode connection is lost
- **Invalid Parameters**: Clear error messages for malformed requests  
- **Process Discovery**: Fallback mechanisms for PID detection
- **Resource Loading**: Fallback to basic prompts when guidance unavailable
- **Context Fetching**: Yiasou prompt works even without taskspace context
- **Test Mode**: Mock responses when `DIALECTIC_TEST_MODE=1` is set
