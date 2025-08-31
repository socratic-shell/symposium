# Communication Protocol

The communication follows a hub-and-spoke pattern with a [central daemon process](./daemon.md). The daemon acts as a message bus, receiving messages from both [MCP servers](./mcp-server.md) and active [extensions](./extension.md) and rebroadcasting them so that everyone can receive them. For detailed message flows and sequence diagrams, see the [Message Flows](./message-flows.md) chapter.

## Protocol

The protocol is very simple. Each message is a JSON structure formatted on a single line. Clients connect to a shared IPC socket. Each message sent by any client is broadcast back to all other clients. Clients are responsible for filtering out their own messages and messages that don't pertain to them.

### Identifying the target of a message

Our [message types](#message-types) include PIDs as needed to help clients filter out messages. For example, when an MCP server initiatives a message, it includes the `shellPid` field to identify which shell it is running inside of. An extension can use this to decide whether that shell is part of its window or some other window within the same IDE. 

## Message Types

### PresentReview Message
```json
{
  "type": "PresentReview",
  "id": "unique-message-id",
  "content": "# Review markdown content...",
  "mode": "replace",
  "baseUri": "/path/to/project",
  "shellPid": 12345
}
```

### Discovery Messages
```json
// MCP server announces presence
{
  "type": "Polo",
  "shellPid": 12345
}

// MCP server announces departure
{
  "type": "Goodbye", 
  "shellPid": 12345
}

// Extension requests discovery
{
  "type": "Marco"
}
```

### Response Messages
```json
// Success response
{
  "id": "unique-message-id",
  "success": true
}

// Error response
{
  "id": "unique-message-id", 
  "success": false,
  "error": "Error description"
}
```

## Implementation References

For specific implementation details, see:
- `server/src/ipc.ts` - IPC client implementation with connection management
- `extension/src/extension.ts` - IPC server setup and message handling
- `server/src/types.ts` - Shared message protocol interfaces
- `server/src/__tests__/ipc.test.ts` - Comprehensive test coverage

The protocol is designed to be simple, reliable, and extensible for future enhancements while maintaining backward compatibility.