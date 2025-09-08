# IPC Communication Protocol Architecture

This document defines the Inter-Process Communication protocol used between Symposium components, including message formats, routing patterns, buffering strategies, and connection management.

## Overview

The IPC protocol enables coordination between:
- **Agent Containers**: AI agents and MCP servers in taskspaces
- **Symposium App**: macOS GUI application for taskspace management
- **Development Tools**: IDEs, terminals, and other development interfaces
- **Host Services**: System-level coordination and resource management

## Protocol Fundamentals

### Message Format

All IPC messages use line-delimited JSON over Unix domain sockets:

```json
{
  "type": "message_type",
  "payload": { /* type-specific data */ },
  "id": "unique_message_id", 
  "timestamp": "2025-01-08T15:30:45Z"
}
```

**Core Fields:**
- `type` (required): Message type identifier for routing and handling
- `payload` (required): Type-specific message content
- `id` (required): Unique identifier for request/response correlation
- `timestamp` (optional): ISO 8601 timestamp for ordering and debugging

### Transport Layer

**Unix Domain Sockets:**
```bash
# Standard socket locations
/tmp/symposium-daemon.sock           # Host-level daemon
/tmp/symposium-taskspace-{id}.sock   # Taskspace-specific daemon
```

**Connection Pattern:**
- **Persistent connections**: Long-lived bidirectional streams
- **Automatic reconnection**: Clients reconnect on socket failure
- **Multiple clients**: Single daemon serves multiple concurrent connections

## Message Types and Routing

### Buffering Strategy

Messages are selectively buffered based on type naming convention:

**Buffered Messages** (prefix: `buffer_`):
- Persist in daemon memory until acknowledged
- Delivered via replay mechanism for reliability
- Essential for state synchronization across reconnections

**Ephemeral Messages** (no prefix):
- Broadcast immediately to connected clients
- No persistence or replay
- Used for real-time updates and control signals

### Standard Message Types

#### Taskspace Management

**`buffer_taskspace_spawn`** - Create new taskspace:
```json
{
  "type": "buffer_taskspace_spawn",
  "payload": {
    "taskspace_id": "abc123",
    "name": "Authentication Refactor", 
    "description": "Modernize OAuth implementation",
    "project_repo": "https://github.com/user/project.git",
    "initial_prompt": "Please analyze the current auth flow",
    "agent_type": "claude-code"
  },
  "id": "spawn_001"
}
```

**`buffer_taskspace_update`** - Update taskspace metadata:
```json
{
  "type": "buffer_taskspace_update", 
  "payload": {
    "taskspace_id": "abc123",
    "name": "OAuth 2.1 Migration",
    "description": "Updated to use PKCE flow"
  },
  "id": "update_001"
}
```

#### Progress Reporting

**`buffer_taskspace_progress`** - Agent progress updates:
```json
{
  "type": "buffer_taskspace_progress",
  "payload": {
    "taskspace_id": "abc123",
    "message": "Analyzing existing authentication middleware",
    "category": "info",          // info, warn, error, milestone, question
    "progress_percent": 15,      // Optional completion percentage
    "details": {                 // Optional structured details
      "files_analyzed": 12,
      "functions_found": 8
    }
  },
  "id": "progress_001"
}
```

**`buffer_taskspace_signal_user`** - Request user attention:
```json
{
  "type": "buffer_taskspace_signal_user",
  "payload": {
    "taskspace_id": "abc123", 
    "message": "Need clarification on password policy requirements",
    "priority": "high",          // low, medium, high, urgent
    "suggested_actions": [
      "Review security requirements document",
      "Check with security team"
    ]
  },
  "id": "signal_001"
}
```

#### Real-time Control

**`agent_interrupt`** - Interrupt running agent:
```json
{
  "type": "agent_interrupt",
  "payload": {
    "taskspace_id": "abc123",
    "reason": "user_requested"   // user_requested, timeout, error
  },
  "id": "interrupt_001"
}
```

**`agent_command`** - Send command to agent:
```json
{
  "type": "agent_command",
  "payload": {
    "taskspace_id": "abc123",
    "command": "Please focus on the database migration logic",
    "inject_method": "tmux_send_keys"  // tmux_send_keys, api_call
  },
  "id": "command_001"
}
```

#### System Coordination

**`system_status`** - System health and resource updates:
```json
{
  "type": "system_status",
  "payload": {
    "host": "localhost",
    "active_taskspaces": 3,
    "memory_usage": "75%",
    "cpu_usage": "45%",
    "available_ports": [10001, 10002, 10003]
  },
  "id": "status_001"
}
```

## Request-Response Pattern

### Response Messages

Responses use the special type `response` and reuse the original message ID:

```json
{
  "type": "response",
  "payload": {
    "status": "success",         // success, error, pending
    "data": { /* response data */ },
    "error": "error description" // Present only if status is error
  },
  "id": "spawn_001"             // Same ID as original request
}
```

### Example Request-Response Flow

**Request:**
```json
{
  "type": "buffer_taskspace_spawn",
  "payload": {
    "taskspace_id": "abc123",
    "name": "New Feature",
    "project_repo": "https://github.com/user/project.git"
  },
  "id": "spawn_001"
}
```

**Response:**
```json
{
  "type": "response", 
  "payload": {
    "status": "success",
    "data": {
      "taskspace_id": "abc123",
      "dev_port": 10001,
      "agent_port": 10002,
      "ssh_config_updated": true
    }
  },
  "id": "spawn_001"
}
```

## Buffering and Replay Mechanism

### Buffer Management

The daemon maintains separate buffers per taskspace:

```typescript
// Daemon buffer structure
interface TaskspaceBuffer {
  taskspace_id: string;
  messages: BufferedMessage[];
  last_replay: Date;
}

interface BufferedMessage {
  message: IPCMessage;
  received_at: Date;
  acknowledged: boolean;
}
```

### Replay Protocol

**Request Replay:**
```json
{
  "type": "buffer_replay",
  "payload": {
    "taskspace_id": "abc123",     // Optional: specific taskspace
    "since": "2025-01-08T10:00:00Z"  // Optional: timestamp filter
  },
  "id": "replay_001"
}
```

**Replay Response:**
```json
{
  "type": "response",
  "payload": {
    "status": "success",
    "data": {
      "messages": [
        {
          "type": "buffer_taskspace_progress",
          "payload": { /* original message payload */ },
          "id": "progress_001",
          "timestamp": "2025-01-08T10:15:30Z"
        }
        // ... more buffered messages
      ],
      "replay_timestamp": "2025-01-08T15:30:45Z"
    }
  },
  "id": "replay_001"
}
```

### Buffer Acknowledgment

After processing replayed messages, clients acknowledge:

```json
{
  "type": "buffer_acknowledge",
  "payload": {
    "taskspace_id": "abc123",
    "replay_timestamp": "2025-01-08T15:30:45Z"
  },
  "id": "ack_001"
}
```

The daemon then clears acknowledged messages from the buffer.

## Connection Management

### Client Registration

Clients register their capabilities and interests:

```json
{
  "type": "client_register",
  "payload": {
    "client_type": "symposium_app",  // symposium_app, agent, ide, cli
    "capabilities": [
      "taskspace_management",
      "progress_display", 
      "user_interaction"
    ],
    "subscriptions": [              // Message types of interest
      "buffer_taskspace_*",
      "system_status"
    ]
  },
  "id": "register_001"
}
```

### Heartbeat Protocol

```json
{
  "type": "heartbeat",
  "payload": {
    "client_id": "symposium_app_001",
    "uptime": 3600,                 // Seconds since connection
    "status": "healthy"
  },
  "id": "heartbeat_001"
}
```

### Connection Events

**Client Connected:**
```json
{
  "type": "client_connected",
  "payload": {
    "client_id": "agent_abc123",
    "client_type": "agent",
    "connected_at": "2025-01-08T15:30:45Z"
  },
  "id": "connect_001"
}
```

**Client Disconnected:**
```json
{
  "type": "client_disconnected", 
  "payload": {
    "client_id": "agent_abc123",
    "disconnect_reason": "connection_lost",  // connection_lost, client_shutdown, daemon_shutdown
    "uptime": 1800
  },
  "id": "disconnect_001"
}
```

## Error Handling and Recovery

### Error Message Format

```json
{
  "type": "error",
  "payload": {
    "error_code": "TASKSPACE_NOT_FOUND",
    "error_message": "Taskspace 'abc123' does not exist",
    "original_message_id": "spawn_001",
    "retry_after": 5000             // Milliseconds before retry suggested
  },
  "id": "error_001"
}
```

### Standard Error Codes

- `TASKSPACE_NOT_FOUND`: Referenced taskspace doesn't exist
- `INSUFFICIENT_RESOURCES`: Cannot allocate requested resources
- `NETWORK_PORT_UNAVAILABLE`: Required network ports in use
- `AUTHENTICATION_FAILED`: Invalid credentials or permissions
- `MESSAGE_FORMAT_INVALID`: Malformed JSON or missing required fields
- `RATE_LIMIT_EXCEEDED`: Too many requests from client

### Reconnection Strategy

```typescript
// Client reconnection logic
interface ReconnectionConfig {
  initial_delay: number;       // Initial delay in milliseconds
  max_delay: number;          // Maximum delay between attempts
  backoff_factor: number;     // Exponential backoff multiplier
  max_attempts: number;       // Maximum reconnection attempts
}

const default_config = {
  initial_delay: 1000,        // 1 second
  max_delay: 30000,          // 30 seconds
  backoff_factor: 2.0,       // Double delay each attempt
  max_attempts: 10           // Give up after 10 attempts
};
```

## Security and Authentication

### Message Authentication

Messages can include authentication tokens:

```json
{
  "type": "buffer_taskspace_spawn",
  "payload": { /* message content */ },
  "id": "spawn_001",
  "auth": {
    "token": "bearer_token_here",
    "client_id": "symposium_app_001"
  }
}
```

### Access Control

Daemon enforces access control based on client type and taskspace ownership:

```typescript
// Access control rules
const access_rules = {
  symposium_app: {
    allowed_types: ["buffer_*", "system_*"],
    taskspace_access: "all"
  },
  agent: {
    allowed_types: ["buffer_taskspace_progress", "buffer_taskspace_signal_user"],
    taskspace_access: "owned_only"
  },
  ide: {
    allowed_types: ["agent_command", "agent_interrupt"],
    taskspace_access: "connected_only"
  }
};
```

## Performance and Scalability

### Message Throughput

- **Target performance**: 1000 messages/second per daemon
- **Buffer limits**: 10,000 messages per taskspace maximum
- **Connection limits**: 100 concurrent client connections

### Memory Management

```typescript
// Buffer cleanup policies
interface BufferPolicy {
  max_messages_per_taskspace: number;    // 10,000
  max_buffer_age_hours: number;          // 24 hours
  cleanup_interval_minutes: number;      // 5 minutes
  max_total_memory_mb: number;          // 100 MB
}
```

### Message Compression

For high-throughput scenarios, messages can use gzip compression:

```json
{
  "type": "buffer_taskspace_progress",
  "payload_compressed": true,
  "payload": "H4sIAAAAAAAAA...",  // gzipped and base64 encoded
  "id": "progress_001"
}
```

This IPC protocol provides robust, scalable communication with reliable message delivery, flexible routing, and comprehensive error handling for complex multi-component coordination.