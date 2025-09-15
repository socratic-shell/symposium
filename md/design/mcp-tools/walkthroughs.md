# Code Walkthrough Tools

## `present_walkthrough`

```rust
// --- Parameters -----------------------
{{#include ../../../socratic-shell/mcp-server/src/types.rs:present_walkthrough_params}}

// --- Tool definition ------------------
{{#include ../../../socratic-shell/mcp-server/src/server.rs:present_walkthrough_tool}}
```

**Supported XML elements**:
- `<comment location="EXPR" icon="ICON">content</comment>` - Code comments at specific locations
- `<gitdiff range="COMMIT_RANGE" />` - Show code changes
- `<action button="TEXT">message</action>` - Interactive buttons
- `<mermaid>diagram</mermaid>` - Architecture diagrams

**Use case**: Create interactive code tours and explanations
