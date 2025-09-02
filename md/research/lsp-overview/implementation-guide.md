# Implementation Guide

## Performance Guidelines

**Message Ordering:**
Responses to requests should be sent in roughly the same order as the requests appear on the server or client side.

**State Management:**
- Servers should handle partial/incomplete requests gracefully
- Use `ContentModified` error for outdated results
- Implement proper cancellation support

**Resource Management:**
- Language servers run in separate processes
- Avoid memory leaks in long-running servers  
- Implement proper cleanup on shutdown

## Error Handling

**Client Responsibilities:**
- Restart crashed servers (with exponential backoff)
- Handle `ContentModified` errors gracefully
- Validate server responses

**Server Responsibilities:**
- Return appropriate error codes
- Handle malformed/outdated requests
- Monitor client process health

## Transport Considerations

**Command Line Arguments:**
```bash
language-server --stdio                    # Use stdio
language-server --pipe=<n>             # Use named pipe/socket
language-server --socket --port=<port>    # Use TCP socket  
language-server --node-ipc                # Use Node.js IPC
language-server --clientProcessId=<pid>   # Monitor client process
```

## Testing Strategies

**Unit Testing:**
- Mock LSP message exchange
- Test individual feature implementations
- Validate message serialization/deserialization

**Integration Testing:**
- End-to-end editor integration
- Multi-document scenarios
- Error condition handling

**Performance Testing:**
- Large file handling
- Memory usage patterns
- Response time benchmarks

## Advanced Topics

### Custom Extensions

**Experimental Capabilities:**
```typescript
interface ClientCapabilities {
  experimental?: {
    customFeature?: boolean;
    vendorSpecificExtension?: any;
  };
}
```

**Custom Methods:**
- Use vendor prefixes: `mycompany/customFeature`
- Document custom protocol extensions
- Ensure graceful degradation

### Security Considerations

**Process Isolation:**
- Language servers run in separate processes
- Limit file system access appropriately  
- Validate all input from untrusted sources

**Content Validation:**
- Sanitize file paths and URIs
- Validate document versions
- Implement proper input validation

### Multi-Language Support

**Language Identification:**
```typescript
interface TextDocumentItem {
  uri: DocumentUri;
  languageId: string; // "typescript", "python", etc.
  version: integer;
  text: string;
}
```

**Document Selectors:**
```typescript
type DocumentSelector = DocumentFilter[];

interface DocumentFilter {
  language?: string;    // "typescript"
  scheme?: string;      // "file", "untitled"  
  pattern?: string;     // "**/*.{ts,js}"
}
```
