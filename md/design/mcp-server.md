# MCP Server

The MCP server acts as a bridge between AI assistants and the VSCode extension. It offers [tools](./tool-interface.md) that AI assistants can use to send messages to the extension (e.g., present-review).

## Lifecycle

MCP servers are started automatically by the AI client. They can be long- or short-lived depending on the whims of the client.

## PID identification

When the MCP server starts, it walks up the process tree to identify the PID for the shell it is running within and the IDE that this shell is contained within. If those PIDs cannot be identified, it returns an error. As described in the [daemon lifecycle](./daemon.md#lifecycle), the server will also spawn a daemon process for the IDE PID if needed.

## Tools offered to clients

See the [Tool Interface](./mcp-tool-interface.md) section.
