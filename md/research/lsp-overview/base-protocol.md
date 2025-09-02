# Base Protocol

## Message Structure

The base protocol consists of a header and a content part (comparable to HTTP). The header and content part are separated by a '\r\n'.

### Header Format
```
Content-Length: <number>\r\n
Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n
\r\n
```

**Required Headers:**
- `Content-Length`: Length of content in bytes (mandatory)
- `Content-Type`: MIME type (optional, defaults to `application/vscode-jsonrpc; charset=utf-8`)

### Content Format

Contains the actual content of the message. The content part of a message uses JSON-RPC to describe requests, responses and notifications.

**Example Message:**
```json
Content-Length: 126\r\n
\r\n
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/completion",
  "params": {
    "textDocument": { "uri": "file:///path/to/file.js" },
    "position": { "line": 5, "character": 10 }
  }
}
```

## JSON-RPC Structure

### Base Message
```typescript
interface Message {
  jsonrpc: string; // Always "2.0"
}
```

### Request Message
```typescript
interface RequestMessage extends Message {
  id: integer | string;
  method: string;
  params?: array | object;
}
```

### Response Message
```typescript
interface ResponseMessage extends Message {
  id: integer | string | null;
  result?: any;
  error?: ResponseError;
}
```

### Notification Message
```typescript
interface NotificationMessage extends Message {
  method: string;
  params?: array | object;
}
```

## Error Handling

**Standard Error Codes:**
- `-32700`: Parse error
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

**LSP-Specific Error Codes:**
- `-32803`: RequestFailed
- `-32802`: ServerCancelled
- `-32801`: ContentModified
- `-32800`: RequestCancelled
