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

**IPC Client Actor**: Transport layer that handles Unix socket connection management, message serialization/deserialization, and forwards parsed `IPCMessage`s via tokio channels.

**IPC Dispatch Actor**: Message router that receives `IPCMessage`s from client actor, routes replies to waiting callers, and coordinates with other actors.

**Stdout Actor**: Simple actor for CLI mode that receives `IPCMessage`s and prints them to stdout.

**Discovery Actor**: Handles marco/polo protocol for server discovery.

**Reference Actor**: Handles code reference storage/retrieval.

**Channel-Based Architecture**:
- Client Actor → tokio::channel → Dispatch Actor (MCP server mode)
- Client Actor → tokio::channel → Stdout Actor (CLI mode)
- Clean separation where each actor has single responsibility
- Actors communicate via typed channels, not shared mutable state

**Benefits**:
- Each actor has a single responsibility and can be tested in isolation
- Message passing eliminates the need for manual lock management
- Clear message flow makes debugging easier
- The daemon module becomes a thin stdio adapter that uses actors internally
- Same client actor works for both MCP server and CLI modes
- Public API remains unchanged, ensuring backward compatibility

# Implementation plan

> What is your implementaton plan?

## Phase 1: Extract Core Dispatch Logic ✅ COMPLETED
1. ~~Extract `IpcActor` from `ipc.rs` as `DispatchActor`~~ **COMPLETED**
2. ~~Move pending reply tracking and message routing to dispatch actor~~ **COMPLETED**
3. ~~Add Actor trait with standardized spawn() pattern~~ **COMPLETED**
4. ~~Improve dispatch methods with timeout and generic return types~~ **COMPLETED**
5. ~~Redesign with trait-based messaging system~~ **COMPLETED**

## Phase 2: Client and Stdio Actors ✅ COMPLETED
1. ~~Implement `ClientActor` with connection management and auto-start logic~~ **COMPLETED**
2. ~~Extract transport logic from `daemon::run_client`~~ **COMPLETED**
3. ~~Create channel-based communication with dispatch actor~~ **COMPLETED**
4. ~~Implement `StdioActor` for CLI mode with bidirectional stdin/stdout~~ **COMPLETED**
5. ~~All actors implement Actor trait with consistent spawn() pattern~~ **COMPLETED**
6. ~~Simplify ClientActor interface with `spawn_client()` function~~ **COMPLETED**

## Phase 3: Integration and Wiring ✅ COMPLETED
1. ~~Refactor `daemon::run_client` to use `ClientActor` + `StdioActor`~~ **COMPLETED**
2. **NEXT**: Update `IPCCommunicator` to use `ClientActor` + `DispatchActor`
3. Wire all actors together with appropriate channels
4. Ensure all existing tests pass

## Phase 4: Migration to Trait-Based Messaging 🚧 IN PROGRESS
1. **NEXT**: Implement `DispatchMessage` traits for existing message types
2. Migrate callers from manual channel management to `.send<M>()` pattern
3. Add type-safe request/reply pairs with compile-time validation

## Phase 5: Server Actor (Future)
1. Extract server-side connection handling if needed
2. Handle incoming connections and message parsing

## Current Status
- **4 actors implemented**: DispatchActor, ClientActor, StdioActor, + others
- **Actor trait**: Standardized spawn pattern across all actors
- **Trait-based messaging**: Type-safe `DispatchMessage` system with automatic reply handling
- **CLI integration complete**: `daemon::run_client` uses actor architecture
- **Ready for MCP integration**: Actors proven in real CLI usage

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
