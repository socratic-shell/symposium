# Reference System Tools

## `expand_reference`

```rust
// --- Parameters -----------------------
{{#include ../../../symposium/mcp-server/src/server.rs:expand_reference_params}}

// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:expand_reference_tool}}
```

**Use case**: Retrieve stored context for compact references. Also retrieves the bootup prompt ("yiasou") and the various guidance files that are embedded (e.g., "main.md").
