# MCP Server Overview

The Symposium MCP server (`symposium-mcp`) provides a comprehensive set of tools for AI assistants to interact with VSCode and coordinate taskspace orchestration.

## Tool Categories

- **[IDE Integration Tools](./mcp-tools/ide-integration.md)** - Get selections and navigate code structure
- **[Code Walkthrough Tools](./mcp-tools/walkthroughs.md)** - Create interactive code tours and explanations  
- **[Synthetic Pull Request Tools](./mcp-tools/synthetic-prs.md)** - Generate and manage code reviews
- **[Taskspace Orchestration Tools](./mcp-tools/taskspace-orchestration.md)** - Create and coordinate collaborative workspaces
- **[Reference System Tools](./mcp-tools/reference-system.md)** - Manage compact reference storage and retrieval

## Architecture

The MCP server operates as a bridge between AI assistants and the VSCode extension:

1. **Process Discovery**: Automatically discovers the parent VSCode process
2. **IPC Communication**: Connects to the daemon message bus via Unix socket
3. **Tool Execution**: Processes MCP tool calls and routes them appropriately
4. **Response Handling**: Returns structured results to the AI assistant

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
- **Test Mode**: Mock responses when `DIALECTIC_TEST_MODE=1` is set
