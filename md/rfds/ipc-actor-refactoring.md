# Elevator pitch

> What are you proposing to change? Bullet points welcome.

* Refactor the complex IPC code in `socratic-shell/mcp-server/src/ipc.rs` into focused Tokio actors following Alice Ryhl's actor pattern
* Split monolithic `IPCCommunicator` into single-responsibility actors that communicate via channels
* Extract daemon communication logic from the `daemon` module into reusable channel-based actors
* Make the system more testable by isolating concerns and enabling actor-level unit testing

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

The current IPC system has several architectural issues:

**Complex State Management**: The `IPCCommunicator` struct manages multiple concerns in a single entity:
- Unix socket connection handling
- Message serialization/deserialization  
- Pending reply tracking with timeouts
- Daemon discovery and reconnection logic
- Manual state synchronization with `Arc<Mutex<>>`

**Mixed Async Patterns**: The code combines different async approaches inconsistently:
- Some functions use manual `Future` implementations
- Others use async/await
- State sharing relies on locks rather than message passing

**Hard-to-Follow Message Flow**: Message routing is embedded within the communicator logic, making it difficult to trace how messages flow through the system.

**Testing Challenges**: The monolithic structure makes it difficult to test individual components in isolation. Mock implementations require recreating the entire IPC infrastructure.

**Recent Bug Example**: We recently fixed a dialog confirmation bug where agents received immediate "success" responses even when users cancelled taskspace deletion. This happened because the complex state management made it hard to track the proper async flow.

# Shiny future

> How will things will play out once this feature exists?

The refactored system will have clean separation of concerns with focused actors:

**IPC Server Actor**: Pure server logic extracted from daemon module, handles incoming connections and message parsing.

**IPC Client Actor**: Pure client logic with reconnection/process launching, maintains connection state.

**IPC Dispatch Actor**: Message router that receives from client, routes replies to waiting callers, forwards messages to other actors.

**Discovery Actor**: Handles marco/polo protocol for server discovery.

**Reference Actor**: Handles code reference storage/retrieval.

**Benefits**:
- Each actor has a single responsibility and can be tested in isolation
- Message passing eliminates the need for manual lock management
- Clear message flow makes debugging easier
- The daemon module becomes a thin stdio adapter that uses actors internally
- Public API remains unchanged, ensuring backward compatibility

# Implementation plan

> What is your implementaton plan?

## Phase 1: Extract Core IPC Logic
1. Implement the `IpcActor` following the pattern outlined in commit `ba81dd7`
2. Extract server/client logic from existing `daemon` module into `IpcServerActor` and `IpcClientActor`
3. Create channel-based communication between actors

## Phase 2: Message Routing
1. Implement `IpcDispatchActor` for message routing
2. Move pending reply tracking from `IPCCommunicator` to the dispatch actor
3. Ensure proper timeout handling and cancellation

## Phase 3: Specialized Actors
1. Extract discovery logic into `DiscoveryActor`
2. Extract reference handling into `ReferenceActor`
3. Wire all actors together with appropriate channels

## Phase 4: Integration
1. Refactor `daemon` module to use actors internally while maintaining stdio interface
2. Update `IPCCommunicator` to be a thin wrapper around actor handles
3. Ensure all existing tests pass

## Actor Communication Pattern
Each actor follows the standard Tokio actor pattern:
- **Actor struct**: Owns state and message receiver, runs the main loop
- **Handle struct**: Provides public API and holds message sender
- **Message enum**: Defines operations the actor can perform

```rust
// Example pattern
enum ActorRequest {
    DoSomething { data: String, reply_tx: oneshot::Sender<Result> },
}

struct Actor {
    receiver: mpsc::Receiver<ActorRequest>,
    // actor-specific state
}

#[derive(Clone)]
struct ActorHandle {
    sender: mpsc::Sender<ActorRequest>,
}
```

## Documentation Updates Required

As implementation progresses, the following design documentation will need updates to reflect the new actor-based architecture:

**[Implementation Overview](../design/implementation-overview.md)**: Add a section describing the actor system as a key internal architectural component of the MCP server, explaining how it improves the codebase's maintainability and testability.

**Internal Architecture Documentation**: Create new documentation (likely in `md/design/mcp-server/` or similar) that details the actor system for developers working on the MCP server internals. This should include actor responsibilities, message flows between actors, and testing approaches.

**Note**: External interfaces and public APIs remain unchanged, so most design documentation (daemon.md, message-flows.md, etc.) should not need updates since the actor refactoring is purely an internal implementation detail.

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## What alternative approaches did you consider, and why did you settle on this one?

**Alternative 1: Incremental refactoring without actors**
We could gradually extract functions and modules without changing the fundamental architecture. However, this wouldn't address the core issues of complex state management and mixed async patterns.

**Alternative 2: Complete rewrite**
We could start from scratch with a new IPC system. However, this would be riskier and take longer, and we'd lose the battle-tested logic that already works.

**Why actors**: The actor pattern provides:
- Natural async boundaries that eliminate lock contention
- Clear ownership of state within each actor
- Testable components that can be mocked easily
- Familiar pattern that follows Rust/Tokio best practices

## How will this maintain backward compatibility?

The public API of the `daemon` module and `IPCCommunicator` will remain unchanged. Internally, these will become thin wrappers that delegate to the appropriate actors. Existing code using these interfaces won't need to change.

## What about performance implications?

Message passing between actors adds some overhead compared to direct function calls. However:
- The overhead is minimal for the message volumes we handle
- Eliminating lock contention may actually improve performance
- The cleaner architecture will make future optimizations easier

## How will error handling work across actors?

Each actor will handle its own errors and communicate failures through result types in messages. The dispatch actor will coordinate error propagation to ensure callers receive appropriate error responses.

# Revision history

* Initial draft - September 18, 2025
