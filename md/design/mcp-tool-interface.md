# MCP Tool Interface

## present_review

```rust
{{#include ../../server/src/server.rs:present_review_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/types.rs:present_review_params}}
```

## present_walkthrough

```rust
{{#include ../../server/src/server.rs:present_walkthrough_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/types.rs:present_walkthrough_params}}
```

## get_selection

```rust
{{#include ../../server/src/server.rs:get_selection_tool}}
```

*No parameters required.*

## ide_operation

```rust
{{#include ../../server/src/server.rs:ide_operation_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/server.rs:ide_operation_params}}
```

## request_review

```rust
{{#include ../../server/src/server.rs:request_review_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/synthetic_pr/mcp_tools.rs:request_review_params}}
```

## update_review

```rust
{{#include ../../server/src/server.rs:update_review_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/synthetic_pr/mcp_tools.rs:update_review_params}}
```

## get_review_status

```rust
{{#include ../../server/src/server.rs:get_review_status_tool}}
```

*No parameters required.*

## expand_reference

```rust
{{#include ../../server/src/server.rs:expand_reference_tool}}
```

**Parameters:**
```rust
{{#include ../../server/src/server.rs:expand_reference_params}}
```
