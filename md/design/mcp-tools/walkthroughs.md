# Code Walkthrough Tools

## `present_walkthrough`

```rust
// --- Parameters -----------------------
{{#include ../../../symposium/mcp-server/src/types.rs:present_walkthrough_params}}

// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:present_walkthrough_tool}}
```

**Supported XML elements**:
- `<comment location="EXPR" icon="ICON">content</comment>` - Code comments at specific locations
- `<action button="TEXT">message</action>` - Interactive buttons
- `<mermaid>diagram</mermaid>` - Architecture diagrams

**Use case**: Create interactive code tours and explanations
