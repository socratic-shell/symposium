# Experiment 1: HTTP + Buffering Daemon

**Status**: Planning  
**Started**: 2025-01-08  
**Objective**: Evolve the current Unix socket daemon to use HTTP communication and implement message buffering/replay functionality.

## Core Hypothesis

We can enhance the existing daemon to:
1. Accept HTTP requests instead of Unix socket connections
2. Buffer messages when clients are disconnected  
3. Replay buffered messages when clients reconnect
4. Maintain compatibility with existing MCP tools and Symposium app

This validates the communication model needed for containerized agents before adding container complexity.

## Success Criteria

### Minimum Viable Success
- [ ] Daemon accepts HTTP requests on localhost port
- [ ] MCP server can connect via HTTP instead of Unix socket
- [ ] Symposium app can connect via HTTP instead of Unix socket  
- [ ] All existing functionality (spawn_taskspace, log_progress, etc.) works over HTTP

### Stretch Goals
- [ ] Message buffering: daemon stores messages when no active connections
- [ ] Message replay: when client reconnects, receives all buffered messages
- [ ] Connection persistence tracking across reconnects
- [ ] Message ordering and deduplication

## Technical Approach

### Current State Analysis
Looking at the existing daemon architecture:
- Node.js/TypeScript daemon using Unix domain sockets
- IPC message routing between MCP server and Symposium app
- Socket at `~/.socratic-shell/symposium/daemon.sock`

### HTTP Migration Strategy
```
Phase 1: HTTP Endpoint Addition
├── Add Express.js HTTP server alongside Unix socket
├── Implement HTTP handlers for existing IPC messages
├── Add HTTP client adapters for MCP server and Symposium app
└── Test: HTTP and Unix socket work simultaneously

Phase 2: Buffering Implementation  
├── Add in-memory message buffer per client connection
├── Implement connection state tracking (connected/disconnected)
├── Buffer messages when target client disconnected
└── Replay buffered messages on reconnection

Phase 3: Unix Socket Deprecation
├── Switch MCP server to HTTP-only
├── Switch Symposium app to HTTP-only  
├── Remove Unix socket code
└── Update documentation and configuration
```

### API Design

**Endpoint Structure**:
```
POST /api/messages - Send message to daemon
GET /api/messages/{clientId} - Long-polling for messages
POST /api/connect - Register client connection
DELETE /api/disconnect/{clientId} - Unregister client
GET /api/status - Daemon health and client status
```

**Message Format** (same as current Unix socket):
```typescript
interface IPCMessage {
  type: 'new_taskspace_request' | 'progress_log' | 'user_signal' | 'taskspace_updated';
  payload: any;
  clientId: string;
  timestamp: number;
}
```

### Buffering Architecture
```
Daemon Memory:
├── clients: Map<clientId, ConnectionState>
├── buffers: Map<clientId, IPCMessage[]> 
├── connectionStates: Map<clientId, 'connected' | 'disconnected'>
└── messageHandlers: Map<messageType, Handler>

Message Flow:
1. Message arrives via POST /api/messages
2. Check if target client is connected
3. If connected: forward immediately
4. If disconnected: add to buffer for clientId
5. When client reconnects: replay all buffered messages
```

## Implementation Plan

### Phase 1: HTTP Infrastructure (Week 1)
1. **Add HTTP server to existing daemon**
   - Install Express.js, setup HTTP server on configurable port (default 3737)  
   - Create HTTP message handlers that call existing Unix socket logic
   - Add HTTP client wrapper for MCP server connections

2. **Dual-mode operation**  
   - Daemon supports both Unix socket AND HTTP simultaneously
   - Add configuration flag for connection method
   - Test existing functionality works over HTTP

3. **Update MCP server**
   - Add HTTP client option alongside Unix socket client
   - Configuration to choose connection method
   - Verify all MCP tools work over HTTP

### Phase 2: Buffering Logic (Week 2)  
1. **Connection state tracking**
   - Client registration/deregistration endpoints
   - Track which clients are actively connected
   - Heartbeat or long-polling to detect disconnections

2. **Message buffering**
   - In-memory buffer per client (with size limits)  
   - Buffer messages when target client disconnected
   - Message persistence across daemon restarts (optional stretch)

3. **Replay mechanism**
   - When client reconnects, send all buffered messages in order
   - Mark messages as delivered to avoid re-delivery
   - Handle buffer overflow gracefully

### Phase 3: Migration and Polish (Week 3)
1. **Switch clients to HTTP-only**
   - Update default configuration to use HTTP
   - Remove Unix socket dependencies from MCP server
   - Update Symposium app to use HTTP

2. **Remove Unix socket support**
   - Clean up daemon code to remove Unix socket handling
   - Update documentation and setup instructions
   - Performance testing and optimization

## Open Questions

1. **Port Management**: How do we handle port conflicts? Dynamic port allocation, or fixed port with conflict detection?

2. **Authentication**: Do we need any authentication for localhost HTTP connections, or rely on OS-level security?

3. **Buffer Limits**: What's reasonable buffer size per client? What happens on overflow? (LRU eviction, drop oldest, refuse new messages?)

4. **Message Persistence**: Should buffered messages survive daemon restarts? (Probably not for MVP, but worth considering)

5. **Long-polling vs WebSockets**: Is HTTP long-polling sufficient, or do we need WebSockets for better real-time communication?

## Risk Mitigation

**Risk**: HTTP introduces network security concerns  
**Mitigation**: Bind to localhost only, consider simple token auth if needed

**Risk**: Buffering memory usage grows unbounded  
**Mitigation**: Implement buffer size limits, message TTL, oldest-first eviction

**Risk**: Message ordering issues with buffering  
**Mitigation**: Add sequence numbers, ensure FIFO replay within client buffers

**Risk**: Performance degradation vs Unix sockets  
**Mitigation**: Benchmark HTTP vs Unix socket performance, optimize if significant difference

## Success Validation

**After Phase 1**: All existing Symposium functionality works identically over HTTP as Unix sockets

**After Phase 2**: Can disconnect MCP client, send messages from Symposium app, reconnect MCP client and see all missed messages

**After Phase 3**: No functional differences from user perspective, but foundation ready for containerized agents

## Next Steps After Success

This experiment directly enables Experiment 2 (Containerized Agent) because:
- HTTP allows communication across container boundaries
- Buffering handles container restarts and network interruptions  
- Connection state tracking supports dynamic container lifecycle

## Related Documentation

- [IPC Communication and Daemon Architecture](../../design/daemon.md) - Current daemon implementation
- [IPC Communication Protocol](../architecture/ipc-protocol.md) - Future vision for message protocol
- [Experiment 2: Containerized Agent](./experiment-2-containerized-agent.md) - Next experiment building on this