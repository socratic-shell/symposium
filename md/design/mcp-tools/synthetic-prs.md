# Synthetic Pull Request Tools

## `request_review`

```rust
// --- Parameters -----------------------
{{#include ../../../socratic-shell/mcp-server/src/synthetic_pr/mcp_tools.rs:request_review_params}}

// --- Tool definition ------------------
{{#include ../../../socratic-shell/mcp-server/src/server.rs:request_review_tool}}
```

**Use case**: Generate structured code reviews from commits

## `update_review`

```rust
// --- Parameters -----------------------
{{#include ../../../socratic-shell/mcp-server/src/synthetic_pr/mcp_tools.rs:update_review_params}}

// --- Tool definition ------------------
{{#include ../../../socratic-shell/mcp-server/src/server.rs:update_review_tool}}
```

**Use case**: Manage review workflows and collect user feedback

## `get_review_status`

```rust
// --- Tool definition ------------------
{{#include ../../../socratic-shell/mcp-server/src/server.rs:get_review_status_tool}}
```

**Use case**: Check review state and progress
