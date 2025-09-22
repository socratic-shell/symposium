# Synthetic Pull Request Tools ![Planned](https://img.shields.io/badge/status-planned-blue)

## `request_review` ![Planned](https://img.shields.io/badge/status-planned-blue)

*Implementation pending - will generate structured code reviews from commits.*

**Use case**: Generate structured code reviews from commits

## `update_review` ![Planned](https://img.shields.io/badge/status-planned-blue)

*Implementation pending - will update existing code reviews.*

// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:update_review_tool}}
```

**Use case**: Manage review workflows and collect user feedback

## `get_review_status`

```rust
// --- Tool definition ------------------
{{#include ../../../symposium/mcp-server/src/server.rs:get_review_status_tool}}
```

**Use case**: Check review state and progress
