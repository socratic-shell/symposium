# Daemon Message Bus

The daemon message bus serves as the central communication hub that routes messages between MCP servers and VSCode extensions across multiple windows. It eliminates the need for direct connections while enabling intelligent message routing based on terminal capabilities.

## Binary

The [MCP server](./mcp-server.md) and daemon are packaged in the same binary. Daemons are started by passing the `daemon` parameter on the command line.

## Lifecycle

When an [MCP server](./mcp-server.md) starts, it identifies the PID of the IDE it is running inside of and attempts to start a daemon for that PID as a new process. If a daemon is already running, then this has no effect.

The daemon runs as a root process. It periodically monitors the IDE PID to see whether the IDE has exited. If the IDE has terminated, the daemon will automatically terminate.

## IPC communication

Daemons bind an IPC socket whose name is determined by the PID of the IDE (given on the command line).
The [IPC protocol](./protocol.md) is defined separately. Daemons themselves don't understand the protocol, they simple accept newline-delimited messages and re-broadcast them to all connected clients. Daemons are intentionally very simple.
