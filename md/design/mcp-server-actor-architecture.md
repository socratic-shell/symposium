# MCP Server Actor Architecture

{RFD:ipc-actor-refactoring}

*Internal architecture for the MCP server's actor-based IPC system*

## Overview

The MCP server uses a focused actor architecture following Alice Ryhl's Tokio actor pattern to handle IPC communication. This replaces the previous monolithic `IPCCommunicator` with specialized actors that communicate via message passing channels.

## Actor Responsibilities

### Dispatch Actor
**Purpose**: Message routing and reply correlation  
**Location**: `src/actor/dispatch.rs`

- Routes incoming `IPCMessage`s to appropriate handlers
- Tracks pending replies with timeout management
- Correlates responses with waiting callers
- Eliminates shared mutable state through message passing

```rust
// Core message types
enum DispatchRequest {
    SendMessage { message: IPCMessage, reply_tx: Option<oneshot::Sender<serde_json::Value>> },
    CancelReply { id: String },
}
```

### Client Actor
**Purpose**: Transport layer for daemon communication  
**Location**: `src/actor/client.rs` *(planned)*

- Manages Unix socket connections with retry logic
- Auto-starts daemon process when needed
- Serializes/deserializes messages to/from `IPCMessage`
- Forwards parsed messages via tokio channels

**Message Flow**:
```
Unix Socket → ClientActor → parse → tokio::channel → DispatchActor
DispatchActor → tokio::channel → ClientActor → serialize → Unix Socket
```

### Stdout Actor
**Purpose**: CLI output for daemon client mode  
**Location**: `src/actor/stdout.rs` *(planned)*

- Receives `IPCMessage`s from client actor
- Serializes messages back to JSON
- Prints to stdout for CLI consumption

**Usage**: When `daemon::run_client()` is called, it wires `ClientActor` → `StdoutActor` instead of `ClientActor` → `DispatchActor`.

### Dispatch Actor
**Purpose**: Message routing and Marco/Polo discovery  
**Location**: `src/actor/dispatch.rs`

- Routes messages to appropriate handlers based on type
- Handles Marco/Polo discovery protocol inline
- Manages message bus coordination
- Responds to marco messages with polo

### Reference Actor
**Purpose**: Code reference storage and retrieval  
**Location**: `src/actor/reference.rs`

- Stores code references for later retrieval
- Manages reference lifecycle
- Provides lookup capabilities

## Architecture Patterns

### Actor Structure
Each actor follows the standard Tokio pattern:

```rust
// Message enum defining operations
enum ActorRequest {
    DoSomething { data: String, reply_tx: oneshot::Sender<Result> },
}

// Actor struct owning state and message receiver
struct Actor {
    receiver: mpsc::Receiver<ActorRequest>,
    // actor-specific state
}

// Handle providing public API
#[derive(Clone)]
struct ActorHandle {
    sender: mpsc::Sender<ActorRequest>,
}
```

### Channel-Based Communication
Actors communicate exclusively through typed channels:

- **mpsc channels**: For actor request/response patterns
- **oneshot channels**: For reply correlation
- **broadcast channels**: For pub/sub patterns (if needed)

### Error Handling
- Each actor handles its own errors internally
- Failures are communicated through result types in messages
- Actors can restart independently without affecting others

## Integration Points

### MCP Server Mode
```
VSCode Extension ↔ Unix Socket ↔ ClientActor ↔ DispatchActor ↔ MCP Handlers
```

### CLI Daemon Mode
```
stdin → daemon::run_client → ClientActor ↔ StdoutActor → stdout
```

### IPCCommunicator Compatibility
The existing `IPCCommunicator` becomes a thin wrapper:

```rust
impl IPCCommunicator {
    // Delegates to ClientActor + DispatchActor handles
    pub async fn send_message(&self, msg: IPCMessage) -> Result<serde_json::Value> {
        self.dispatch_handle.send_message_with_reply(msg).await
    }
}
```

## Testing Strategy

### Unit Testing
- Each actor can be tested in isolation
- Mock channels for testing message flows
- Timeout and error scenarios easily testable

### Integration Testing
- Wire actors together in test configurations
- Test complete message flows end-to-end
- Verify proper cleanup and shutdown

## Migration Path

1. **Phase 1** ✅: Extract dispatch actor from `ipc.rs`
2. **Phase 2**: Implement client actor and stdout actor
3. **Phase 3**: Refactor `daemon::run_client` to use actors
4. **Phase 4**: Update `IPCCommunicator` to use actor handles
5. **Phase 5**: Remove legacy code and add comprehensive tests

## Benefits

- **Testability**: Each actor can be unit tested independently
- **Maintainability**: Clear separation of concerns and responsibilities  
- **Reliability**: Message passing eliminates race conditions and lock contention
- **Flexibility**: Easy to add new actors or modify message flows
- **Performance**: Async message passing scales better than shared mutable state

## Future Considerations

- **Server Actor**: May be needed if we handle incoming connections differently
- **Metrics Actor**: Could collect performance and health metrics
- **Configuration Actor**: Could manage dynamic configuration updates
- **Supervision**: Consider actor supervision patterns for fault tolerance
