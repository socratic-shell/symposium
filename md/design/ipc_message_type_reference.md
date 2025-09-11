# IPC Message type reference

For each message type that is sent in the record we record

- purpose
- expected payload (or "varies")
- expected response (if any)
- sent by (extension, MCP server, symposium app, etc)
- any other relevant details

## `response`

Response messages are special. They are sent in response to other messages and their fields are determined in response to that message type:

* the `id` is equal to the `id` of the message being responding to
* the `payload` type depends on the message being responded to

**Sent by**: All components (VSCode extension, MCP server, Symposium app)
**Purpose**: Acknowledge and respond to incoming requests
**Payload**: Varies based on the original message type

## `marco`

**Sent by**: VSCode extension
**Purpose**: Discovery broadcast to find active MCP servers ("who's out there?")
**Payload**: `{}` (empty object)
**Expected response**: `polo` messages from active MCP servers
**Notes**: Uses simplified sender format (no full MessageSender object)

## `polo`

**Sent by**: MCP server
**Purpose**: Response to `marco` discovery messages
**Payload**: Server identification information
**Expected response**: None (broadcast response)

## `store_reference`

**Sent by**: VSCode extension
**Purpose**: Store code references for later expansion by agents
**Payload**: 
```typescript
{
    relativePath: string;
    selectedText: string;
    selectionRange: {
        start: { line: number; column: number };
        end: { line: number; column: number };
    };
}
```
**Expected response**: `response` with success confirmation
**Target**: MCP server

## `get_taskspace_state`

**Sent by**: VSCode extension
**Purpose**: Query the current state of a taskspace
**Payload**:
```typescript
{
    taskspace_uuid: string;
}
```
**Expected response**: `response` with `TaskspaceStateResponse`
**Target**: Symposium app

## `register_taskspace_window`

**Sent by**: VSCode extension
**Purpose**: Register a VSCode window with a specific taskspace
**Payload**:
```typescript
{
    taskspace_uuid: string;
}
```
**Expected response**: `response` with success confirmation
**Target**: Symposium app

## `present_walkthrough`

**Sent by**: MCP server
**Purpose**: Display interactive code walkthrough in VSCode
**Payload**:
```typescript
{
    content: string;        // Markdown content with XML elements
    baseUri: string;        // Base directory for file references
}
```
**Expected response**: None (display command)
**Target**: VSCode extension

## `log`

**Sent by**: MCP server
**Purpose**: Send log messages to VSCode output channel
**Payload**:
```typescript
{
    level: 'info' | 'error' | 'debug';
    message: string;
}
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
```typescript
{
    feedback_type: 'comment' | 'complete_review';
    // Additional fields vary by feedback type
}
```
**Expected response**: `response` with acknowledgment
**Target**: MCP server

## `spawn_taskspace`

**Sent by**: MCP server
**Purpose**: Request creation of a new taskspace
**Payload**: Taskspace creation parameters
**Expected response**: `response` with taskspace info
**Target**: Symposium app

## `log_progress`

**Sent by**: MCP server
**Purpose**: Report progress with visual indicators
**Payload**:
```typescript
{
    message: string;
    category: 'info' | 'warn' | 'error' | 'milestone' | 'question';
}
```
**Expected response**: None (display command)
**Target**: Symposium app

## `signal_user`

**Sent by**: MCP server
**Purpose**: Request user attention for assistance
**Payload**:
```typescript
{
    message: string;
}
```
**Expected response**: None (notification)
**Target**: Symposium app

## `update_taskspace`

**Sent by**: MCP server
**Purpose**: Update taskspace name and description
**Payload**: Taskspace update data
**Expected response**: `response` with success confirmation
**Target**: Symposium app

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

