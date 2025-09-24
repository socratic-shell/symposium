# Get Rust Crate Source

## Overview

The `get_rust_crate_source` tool downloads and provides access to Rust crate source code, making it available for agents to examine APIs, examples, and documentation.

## Parameters

- **crate_name** (required): Name of the crate to fetch
- **version** (optional): Semver range (e.g., "1.0", "^1.2", "~1.2.3"). Defaults to version used in current project
- **pattern** (optional): Regex pattern to search within the crate source

## Behavior

1. **Version Resolution**: Matches the version used in your current project when possible
2. **Caching**: Uses cached copy from `~/.cargo` when available
3. **Fallback**: Creates temporary directory if no cached copy exists
4. **Search**: When pattern provided, searches source files and returns matches

## Usage Examples

```
Ask agent: "Can you fetch the serde crate source?"
Ask agent: "Get tokio source and search for 'async fn spawn'"
Ask agent: "Fetch clap version 4.0 source code"
```

## Benefits

- Agents can understand unfamiliar APIs without hallucinating methods
- Access to rustdoc examples and `examples/` directory code
- Reduces trial-and-error when working with complex crates
