# Taskspace Orchestration Tools

## `spawn_taskspace`

```rust
// --- Parameters -----------------------
{{#include ../../../mcp-server/src/server.rs:spawn_taskspace_params}}

// --- Tool definition ------------------
{{#include ../../../mcp-server/src/server.rs:spawn_taskspace_tool}}
```

**Use case**: Create new collaborative workspaces for specific tasks

## `update_taskspace`

```rust
// --- Parameters -----------------------
{{#include ../../../mcp-server/src/server.rs:update_taskspace_params}}

// --- Tool definition ------------------
{{#include ../../../mcp-server/src/server.rs:update_taskspace_tool}}
```

**Use case**: Update taskspace name and description based on user interaction

## `log_progress`

```rust
// --- Parameters -----------------------
{{#include ../../../mcp-server/src/server.rs:log_progress_params}}

// --- Tool definition ------------------
{{#include ../../../mcp-server/src/server.rs:log_progress_tool}}
```

**Progress categories**:

```rust
{{#include ../../../mcp-server/src/types.rs:progress_category}}
```

**Use case**: Keep users informed of agent progress and status

## `signal_user`

```rust
// --- Parameters -----------------------
{{#include ../../../mcp-server/src/server.rs:signal_user_params}}

// --- Tool definition ------------------
{{#include ../../../mcp-server/src/server.rs:signal_user_tool}}
```

**Use case**: Alert users when agents need help or input
