# IPC Message type reference

For each message type that is sent in the record we record

- purpose
- expected payload (or "varies")
- expected response (if any)
- sent by (extension, MCP server, symposium app, etc)
- any other relevant details

## `response`

**Sent by**: All components (VSCode extension, MCP server, Symposium app)

**Purpose**: Acknowledge and respond to incoming requests

**Payload**: Varies based on the original message type

**Notes:** Response messages are special. They are sent in response to other messages and their fields are determined in response to that message type:

* the `id` is equal to the `id` of the message being responding to
* the `payload` type depends on the message being responded to

## `marco`

**Sent by**: VSCode extension

**Purpose**: Discovery broadcast to find active MCP servers ("who's out there?")

**Payload**: `{}` (empty object)

**Expected response**: `polo` messages from active MCP servers

**Notes**: Uses simplified sender format (no full MessageSender object)

## `polo`

**Sent by**: MCP server

**Purpose**: Response to `marco` discovery messages

**Payload**: 
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:polo_payload}}
```

**Expected response**: None (broadcast response)

**Notes**: Server identification comes from the `sender` field in the IPCMessage

## `store_reference`

**Sent by**: VSCode extension

**Purpose**: Store code references for later expansion by agents

**Payload**: 
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:store_reference_payload}}
```
**Expected response**: `response` with success confirmation

**Target**: MCP server

## `get_taskspace_state`

**Sent by**: VSCode extension

**Purpose**: Query the current state of a taskspace

**Payload**:
```typescript
{{#include ../../symposium/vscode-extension/src/extension.ts:taskspace_roll_call_payload}}
```

**Expected response**: `response` with `TaskspaceStateResponse`

**Target**: Symposium app

## `register_taskspace_window`

**Sent by**: VSCode extension

**Purpose**: Register a VSCode window with a specific taskspace

**Payload**:
```typescript
{{#include ../../symposium/vscode-extension/src/extension.ts:register_taskspace_window_payload}}
```

**Expected response**: `response` with success confirmation

**Target**: Symposium app

## `present_walkthrough`

**Sent by**: MCP server

**Purpose**: Display interactive code walkthrough in VSCode

**Payload**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:present_walkthrough_params}}
```

**Expected response**: None (display command)

**Target**: VSCode extension

## `log`

**Sent by**: MCP server

**Purpose**: Send log messages to VSCode output channel

**Payload**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:log_params}}
```

**Expected response**: None (logging command)

**Target**: VSCode extension

## `get_selection`

**Sent by**: MCP server

**Purpose**: Request currently selected text from VSCode editor

**Payload**: `{}` (empty object)

**Expected response**: `response` with selected text or null

**Target**: VSCode extension

## `reload_window`

**Sent by**: Daemon (on shutdown)

**Purpose**: Instruct all VSCode extensions to reload their windows

**Payload**: `{}` (empty object)

**Expected response**: None (command)

**Target**: All connected VSCode extensions

**Notes**: Broadcast message with generic sender (`/tmp` working directory)

## `goodbye`

**Sent by**: MCP server

**Purpose**: Notify that the MCP server is shutting down

**Payload**: `{}` (empty object)

**Expected response**: None (notification)

**Target**: VSCode extension

## `resolve_symbol_by_name`

**Sent by**: MCP server

**Purpose**: Find symbol definitions by name using LSP

**Payload**:
```typescript
{
    symbol_name: string;
}
```

**Expected response**: `response` with `Vec<ResolvedSymbol>`

**Target**: VSCode extension

## `find_all_references`

**Sent by**: MCP server

**Purpose**: Find all references to a symbol using LSP

**Payload**:
```typescript
{
    symbol_name: string;
}
```
**Expected response**: `response` with `Vec<FileLocation>`

**Target**: VSCode extension

## `create_synthetic_pr`

**Sent by**: MCP server

**Purpose**: Create a new synthetic pull request in VSCode

**Payload**: Synthetic PR creation data

**Expected response**: `response` with PR ID

**Target**: VSCode extension

## `update_synthetic_pr`

**Sent by**: MCP server

**Purpose**: Update an existing synthetic pull request

**Payload**: Synthetic PR update data

**Expected response**: `response` with success confirmation

**Target**: VSCode extension

## `user_feedback`

**Sent by**: VSCode extension

**Purpose**: Send user feedback (comments, review completion) to MCP server

**Payload**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:user_feedback_payload}}
```
**Expected response**: `response` with acknowledgment

**Target**: MCP server

## `spawn_taskspace`

**Sent by**: MCP server

**Purpose**: Request creation of a new taskspace

**Payload**: 
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:spawn_taskspace_payload}}
```
**Expected response**: `response` with taskspace info

**Target**: Symposium app

## `log_progress`

**Sent by**: MCP server

**Purpose**: Report progress with visual indicators

**Payload**:

```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:log_progress_payload}}
```
**Expected response**: None (display command)

**Target**: Symposium app

## `signal_user`

**Sent by**: MCP server

**Purpose**: Request user attention for assistance

**Payload**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:signal_user_payload}}
```

**Expected response**: None (notification)

**Target**: Symposium app

## `update_taskspace`

**Sent by**: MCP server

**Purpose**: Update taskspace name and description

**Payload**: 
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:update_taskspace_payload}}
```
**Expected response**: `response` with success confirmation

**Target**: Symposium app

## `delete_taskspace`

{RFD:taskspace-deletion-dialog-confirmation}

**Sent by**: MCP server

**Purpose**: Request deletion of the current taskspace with user confirmation

**Payload**: 
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:delete_taskspace_payload}}
```

**Expected response**: `response` with success confirmation (sent after user confirms) or error (if user cancels)

**Target**: Symposium app

**Notes**: This message triggers a confirmation dialog. The response is deferred until the user either confirms or cancels the deletion. If confirmed, the taskspace is deleted and a success response is sent. If cancelled, an error response is sent with the message "Taskspace deletion was cancelled by user".

## `taskspace_roll_call`

**Sent by**: Symposium app

**Purpose**: Broadcast to discover active taskspaces for window registration

**Payload**: `{}` (empty object)

**Expected response**: Taskspace registration responses

**Target**: All components (broadcast)

## Message Routing

Messages are routed based on sender information:

- **Directory matching**: Messages are delivered to extensions whose workspace contains the sender's working directory
- **PID matching**: When `shellPid` is provided, messages are delivered to extensions that have a terminal with that PID
- **Taskspace routing**: Messages with `taskspaceUuid` can be routed to specific taskspace-aware components

## Core IPC Types

The IPC message format is consistent across all components:

### IPCMessage Structure

**Rust (MCP Server)**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:ipc_message}}
```

**TypeScript (VSCode Extension)**:
```typescript
{{#include ../../symposium/vscode-extension/src/ipc.ts:ipc_message}}
```

**Swift (Symposium App)**:
*Note: Swift implementation exists but not currently documented with anchors.*

### MessageSender Structure

**Rust (MCP Server)**:
```rust,no_run,noplayground
{{#include ../../symposium/mcp-server/src/types.rs:message_sender}}
```

**TypeScript (VSCode Extension)**:
```typescript
{{#include ../../symposium/vscode-extension/src/ipc.ts:message_sender}}
```

**Swift (Symposium App)**:
*Note: Swift implementation exists but not currently documented with anchors.*

