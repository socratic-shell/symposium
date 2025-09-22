# The Symposium Reference System

The symposium reference system is a **generic key-value store** that allows VSCode extensions to share arbitrary context data with AI assistants.

## Core Concept

Extensions create compact references like `<symposium-ref id="uuid"/>` and store arbitrary JSON context. AI agents expand these references to get the JSON and interpret it contextually based on its self-documenting structure.

**Key insight**: There is no fixed schema. The system stores `(uuid, arbitrary_json_value)` pairs where the JSON structure is determined by the extension and interpreted by the receiving agent.

## Message Format

The payload structure for `store_reference` messages:

**TypeScript (Extension):**
```typescript
{{#include ../../symposium/vscode-extension/src/extension.ts:store_reference_payload}}
```

**Rust (MCP Server):**
```rust
{{#include ../../symposium/mcp-server/src/types.rs:store_reference_payload}}
```

## Usage Examples

```typescript
// Code selection context:
store_reference("uuid-1", {
  relativePath: "src/auth.ts",
  selectionRange: { start: {line: 10, column: 5}, end: {line: 15, column: 2} },
  selectedText: "function validateToken() { ... }"
});

// File reference context:
store_reference("uuid-2", {
  filePath: "README.md", 
  type: "documentation"
});

// Custom application context:
store_reference("uuid-3", {
  queryType: "database_schema",
  tableName: "users", 
  fields: ["id", "email", "created_at"]
});
```

## Implementation

**Storage**: The MCP server stores references as `HashMap<String, serde_json::Value>` where the key is the UUID and the value is arbitrary JSON.

**Retrieval**: The `expand_reference` MCP tool returns the stored JSON value for AI agents to interpret contextually.

## Current Issue

The Rust code tries to deserialize arbitrary JSON into a rigid `ReferenceContext` struct instead of storing it as generic `serde_json::Value`. This breaks the intended generic architecture.

**Solution**: Use `serde_json::Value` throughout the storage and retrieval pipeline.