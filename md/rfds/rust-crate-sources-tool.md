# Elevator pitch

> Integrate Rust crate source exploration capabilities directly into the Socratic Shell MCP server through a unified `get_rust_sources` tool that provides seamless access to crate extraction and optional pattern-based searching.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Currently, developers who want to explore Rust crate source code and examples face several friction points:

- **Manual Process**: Must manually download and extract crates using `cargo` or external tools
- **Fragmented Tooling**: Separate tools for crate extraction vs. searching within source
- **Poor IDE Integration**: External tools lack context awareness and seamless IDE integration
- **Multiple MCP Servers**: The standalone `eg` MCP server requires separate configuration and management

This creates a disjointed experience where developers must context-switch between their IDE and external tools, losing the flow of their development work.

# Shiny future

> How will things will play out once this feature exists?

Developers working in Socratic Shell will have seamless access to Rust crate exploration:

- **Unified Interface**: Single `get_rust_sources` tool handles both extraction and searching
- **IDE Integration**: Results appear directly in the IDE with proper formatting and links
- **Intelligent Responses**: Tool returns only relevant fields (search results only when pattern provided)
- **Cached Performance**: Extracted crates are cached to avoid redundant downloads
- **Rich Context**: Search results include surrounding code lines for better understanding

Example workflows:
- `get_rust_sources(crate_name: "tokio")` → extracts and returns path info
- `get_rust_sources(crate_name: "tokio", pattern: "spawn")` → extracts, searches, and returns matches

# Implementation plan

> What is your implementaton plan?

## Phase 1: Core Integration ✅ (Completed)
1. Copy `eg` library source into `socratic-shell/mcp-server/src/eg/`
2. Add required dependencies to Cargo.toml
3. Implement unified `get_rust_sources` tool with conditional response fields
4. Fix import paths and module structure

## Phase 2: Testing and Documentation
1. Create comprehensive test suite for the tool
2. Update user documentation with usage examples
3. Create migration guide for existing standalone `eg` users
4. Performance testing and optimization

## Phase 3: Enhanced Features (Future)
1. Version selection support (currently uses latest)
2. Configurable context lines for search results
3. Search scope options (examples only vs. all source)
4. Integration with other Socratic Shell tools for enhanced workflows

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## What alternative approaches did you consider, and why did you settle on this one?

**Separate MCP Server**: Keep the standalone `eg` MCP server
- *Pros*: Clean separation of concerns, independent deployment
- *Cons*: Increased complexity, multiple server management, poor integration with other Socratic Shell tools

**External Tool Integration**: Shell out to existing tools like `cargo` 
- *Pros*: Leverages existing tools, minimal development
- *Cons*: Poor IDE integration, complex setup, limited customization, no caching

**LSP Extension**: Implement as Language Server Protocol extension
- *Pros*: Standard protocol, good IDE support
- *Cons*: Limited to language-specific operations, doesn't fit the crate exploration use case well

We settled on direct integration because it provides the best user experience with unified tooling and seamless IDE integration.

## Why copy the source instead of using it as a dependency?

Direct source integration allows us to:
- Eliminate external dependency management complexity
- Enable future customization specific to Socratic Shell needs
- Simplify deployment and maintenance
- Archive the standalone repository once migration is complete

## How does the conditional response format work?

The tool returns different response structures based on whether a search pattern is provided:

- **No pattern**: Returns only crate metadata and extraction path
- **With pattern**: Returns metadata, path, plus `example_matches` and `other_matches` arrays

This keeps responses clean and relevant to the specific use case.

## What happens to existing users of the standalone eg MCP server?

Migration path:
1. Update MCP client configuration to use Socratic Shell MCP server
2. Change tool calls from `search_crate_examples`/`get_crate_source` to `get_rust_sources`
3. Update parameter structure to use unified interface
4. Archive standalone `eg` repository once migration is complete

# Revision history

- 2025-09-17: Initial RFD creation with completed Phase 1 implementation
