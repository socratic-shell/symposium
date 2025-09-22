# IDE Integration Tools

## `get_selection`

```rust
// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:get_selection_tool}}
```

**Returns**: `{ selectedText: string | null }`  
**Use case**: Retrieve user-selected code for analysis or modification

## `ide_operation`

```rust
// --- Parameters -----------------------
{{#include ../../../symposium/mcp-server/src/server.rs:ide_operation_params}}

// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:ide_operation_tool}}
```

**Common Dialect functions**:
- `findDefinitions("symbol")` - Find where a symbol is defined
- `findReferences("symbol")` - Find all uses of a symbol  
- `search("file.rs", "pattern")` - Search file for regex pattern
- `search("dir", "pattern", ".rs")` - Search directory for pattern in specific file types

**Use case**: Navigate code structure, find definitions, search for patterns
