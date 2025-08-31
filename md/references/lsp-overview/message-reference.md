# Message Reference

## Message Types

### Request/Response Pattern

**Client-to-Server Requests:**
- `initialize` - Server initialization
- `textDocument/hover` - Get hover information
- `textDocument/completion` - Get code completions
- `textDocument/definition` - Go to definition

**Server-to-Client Requests:**
- `client/registerCapability` - Register new capabilities
- `workspace/configuration` - Get configuration settings
- `window/showMessageRequest` - Show message with actions

### Notification Pattern

**Client-to-Server Notifications:**
- `initialized` - Initialization complete
- `textDocument/didOpen` - Document opened
- `textDocument/didChange` - Document changed
- `textDocument/didSave` - Document saved
- `textDocument/didClose` - Document closed

**Server-to-Client Notifications:**
- `textDocument/publishDiagnostics` - Send diagnostics
- `window/showMessage` - Display message
- `telemetry/event` - Send telemetry data

### Special Messages

**Dollar Prefixed Messages:**
Notifications and requests whose methods start with '$/' are messages which are protocol implementation dependent and might not be implementable in all clients or servers.

Examples:
- `$/cancelRequest` - Cancel ongoing request
- `$/progress` - Progress reporting
- `$/setTrace` - Set trace level

## Capabilities System

Not every language server can support all features defined by the protocol. LSP therefore provides 'capabilities'. A capability groups a set of language features.

### Capability Exchange

**During Initialization:**
1. Client announces capabilities in `initialize` request
2. Server announces capabilities in `initialize` response
3. Both sides adapt behavior based on announced capabilities

### Client Capabilities Structure

```typescript
interface ClientCapabilities {
  workspace?: WorkspaceClientCapabilities;
  textDocument?: TextDocumentClientCapabilities;
  window?: WindowClientCapabilities;
  general?: GeneralClientCapabilities;
  experimental?: any;
}
```

**Key Client Capabilities:**
- `textDocument.hover.dynamicRegistration` - Support dynamic hover registration
- `textDocument.completion.contextSupport` - Support completion context
- `workspace.workspaceFolders` - Multi-root workspace support
- `window.workDoneProgress` - Progress reporting support

### Server Capabilities Structure

```typescript
interface ServerCapabilities {
  textDocumentSync?: TextDocumentSyncKind | TextDocumentSyncOptions;
  completionProvider?: CompletionOptions;
  hoverProvider?: boolean | HoverOptions;
  definitionProvider?: boolean | DefinitionOptions;
  referencesProvider?: boolean | ReferenceOptions;
  documentSymbolProvider?: boolean | DocumentSymbolOptions;
  workspaceSymbolProvider?: boolean | WorkspaceSymbolOptions;
  codeActionProvider?: boolean | CodeActionOptions;
  // ... many more
}
```

### Dynamic Registration

Servers can register/unregister capabilities after initialization:

```typescript
// Register new capability
client/registerCapability: {
  registrations: [{
    id: "uuid",
    method: "textDocument/willSaveWaitUntil",
    registerOptions: { documentSelector: [{ language: "javascript" }] }
  }]
}

// Unregister capability
client/unregisterCapability: {
  unregisterations: [{ id: "uuid", method: "textDocument/willSaveWaitUntil" }]
}
```

## Lifecycle Management

### Initialization Sequence

1. **Client → Server: `initialize` request**
   ```typescript
   interface InitializeParams {
     processId: integer | null;
     clientInfo?: { name: string; version?: string; };
     rootUri: DocumentUri | null;
     initializationOptions?: any;
     capabilities: ClientCapabilities;
     workspaceFolders?: WorkspaceFolder[] | null;
   }
   ```

2. **Server → Client: `initialize` response**
   ```typescript
   interface InitializeResult {
     capabilities: ServerCapabilities;
     serverInfo?: { name: string; version?: string; };
   }
   ```

3. **Client → Server: `initialized` notification**
   - Signals completion of initialization
   - Server can now send requests to client

### Shutdown Sequence

1. **Client → Server: `shutdown` request**
   - Server must not accept new requests (except `exit`)
   - Server should finish processing ongoing requests

2. **Client → Server: `exit` notification**
   - Server should exit immediately
   - Exit code: 0 if shutdown was called, 1 otherwise

### Process Monitoring

**Client Process Monitoring:**
- Server can monitor client process via `processId` from initialize
- Server should exit if client process dies

**Server Crash Handling:**
- Client should restart crashed servers
- Implement exponential backoff to prevent restart loops

## Document Synchronization

Client support for textDocument/didOpen, textDocument/didChange and textDocument/didClose notifications is mandatory in the protocol and clients can not opt out supporting them.

### Text Document Sync Modes

```typescript
enum TextDocumentSyncKind {
  None = 0,        // No synchronization
  Full = 1,        // Full document sync on every change
  Incremental = 2  // Incremental sync (deltas only)
}
```

### Document Lifecycle

#### Document Open
```typescript
textDocument/didOpen: {
  textDocument: {
    uri: "file:///path/to/file.js",
    languageId: "javascript", 
    version: 1,
    text: "console.log('hello');"
  }
}
```

#### Document Change
```typescript
textDocument/didChange: {
  textDocument: { uri: "file:///path/to/file.js", version: 2 },
  contentChanges: [{
    range: { start: { line: 0, character: 12 }, end: { line: 0, character: 17 } },
    text: "world"
  }]
}
```

**Change Event Types:**
- **Full text**: Replace entire document
- **Incremental**: Specify range and replacement text

#### Document Save
```typescript
// Optional: Before save
textDocument/willSave: {
  textDocument: { uri: "file:///path/to/file.js" },
  reason: TextDocumentSaveReason.Manual
}

// Optional: Before save with text edits
textDocument/willSaveWaitUntil → TextEdit[]

// After save
textDocument/didSave: {
  textDocument: { uri: "file:///path/to/file.js" },
  text?: "optional full text"
}
```

#### Document Close
```typescript
textDocument/didClose: {
  textDocument: { uri: "file:///path/to/file.js" }
}
```

### Position Encoding

Prior to 3.17 the offsets were always based on a UTF-16 string representation. Since 3.17 clients and servers can agree on a different string encoding representation (e.g. UTF-8).

**Supported Encodings:**
- `utf-16` (default, mandatory)
- `utf-8` 
- `utf-32`

**Position Structure:**
```typescript
interface Position {
  line: uinteger;     // Zero-based line number
  character: uinteger; // Zero-based character offset
}

interface Range {
  start: Position;
  end: Position;
}
```

## Workspace Features

### Multi-Root Workspaces

```typescript
workspace/workspaceFolders → WorkspaceFolder[] | null

interface WorkspaceFolder {
  uri: URI;
  name: string;
}

// Notification when folders change
workspace/didChangeWorkspaceFolders: DidChangeWorkspaceFoldersParams
```

### Configuration Management

```typescript
// Server requests configuration from client
workspace/configuration: ConfigurationParams → any[]

interface ConfigurationItem {
  scopeUri?: URI;     // Scope (file/folder) for the setting
  section?: string;   // Setting name (e.g., "typescript.preferences")
}

// Client notifies server of configuration changes
workspace/didChangeConfiguration: DidChangeConfigurationParams
```

### File Operations

#### File Watching
```typescript
workspace/didChangeWatchedFiles: DidChangeWatchedFilesParams

interface FileEvent {
  uri: DocumentUri;
  type: FileChangeType; // Created | Changed | Deleted
}
```

#### File System Operations
```typescript
// Before operations (can return WorkspaceEdit)
workspace/willCreateFiles: CreateFilesParams → WorkspaceEdit | null
workspace/willRenameFiles: RenameFilesParams → WorkspaceEdit | null  
workspace/willDeleteFiles: DeleteFilesParams → WorkspaceEdit | null

// After operations (notifications)
workspace/didCreateFiles: CreateFilesParams
workspace/didRenameFiles: RenameFilesParams
workspace/didDeleteFiles: DeleteFilesParams
```

### Command Execution

```typescript
workspace/executeCommand: ExecuteCommandParams → any

interface ExecuteCommandParams {
  command: string;           // Command identifier
  arguments?: any[];         // Command arguments
}

// Server applies edits to workspace
workspace/applyEdit: ApplyWorkspaceEditParams → ApplyWorkspaceEditResult
```

## Window Features

### Message Display

#### Show Message (Notification)
```typescript
window/showMessage: ShowMessageParams

interface ShowMessageParams {
  type: MessageType; // Error | Warning | Info | Log | Debug
  message: string;
}
```

#### Show Message Request
```typescript
window/showMessageRequest: ShowMessageRequestParams → MessageActionItem | null

interface ShowMessageRequestParams {
  type: MessageType;
  message: string;
  actions?: MessageActionItem[]; // Buttons to show
}
```

#### Show Document
```typescript
window/showDocument: ShowDocumentParams → ShowDocumentResult

interface ShowDocumentParams {
  uri: URI;
  external?: boolean;    // Open in external program
  takeFocus?: boolean;   // Focus the document
  selection?: Range;     // Select range in document
}
```

### Progress Reporting

#### Work Done Progress
```typescript
// Server creates progress token
window/workDoneProgress/create: WorkDoneProgressCreateParams → void

// Report progress using $/progress
$/progress: ProgressParams<WorkDoneProgressBegin | WorkDoneProgressReport | WorkDoneProgressEnd>

// Client can cancel progress
window/workDoneProgress/cancel: WorkDoneProgressCancelParams
```

#### Progress Reporting Pattern
```typescript
// Begin
{ kind: "begin", title: "Indexing", cancellable: true, percentage: 0 }

// Report
{ kind: "report", message: "Processing file.ts", percentage: 25 }

// End  
{ kind: "end", message: "Indexing complete" }
```

### Logging & Telemetry

```typescript
window/logMessage: LogMessageParams     // Development logs
telemetry/event: any                   // Usage analytics
```

## Version History

### LSP 3.17 (Current)
Major new feature are: type hierarchy, inline values, inlay hints, notebook document support and a meta model that describes the 3.17 LSP version.

**Key Features:**
- Type hierarchy support
- Inline value provider
- Inlay hints
- Notebook document synchronization
- Diagnostic pull model
- Position encoding negotiation

### LSP 3.16
**Key Features:**
- Semantic tokens
- Call hierarchy
- Moniker support
- File operation events
- Linked editing ranges
- Code action resolve

### LSP 3.15
**Key Features:**
- Progress reporting
- Selection ranges
- Signature help context

### LSP 3.0
**Breaking Changes:**
- Client capabilities system
- Dynamic registration
- Workspace folders
- Document link support
