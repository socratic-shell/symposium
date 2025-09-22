# Elevator pitch

> What are you proposing to change?

Extend Symposium with a new `get_rust_crate_source` MCP tool that will direct agents to the sources from Rust crates and help to find examples of code that uses particular APIs.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

When Rust developers ask an agent to use a crate that they do not know from their training data, agents typically hallucinate plausible-seeming (but in fact nonexistent) APIs. The easiest way to help an agent get started is to find example code and/or to point them at the crate source, which currently requires manual investigation or the use of a separate (and proprietary) MCP server like context7.

## Leverage Rust's existing `examples` and rustdoc patterns

Cargo has conventions for giving example source code:

* many crates use the `examples` directory
* other crates include (tested) examples from rustdoc

Our MCP server will leverage those sources.

# What we propose to do about it

> What are you proposing to improve the situation?

Integrate Rust crate source exploration capabilities directly into the Symposium MCP server through a unified `get_rust_crate_source` tool that:

- **Extracts crate sources** to a local cache directory for exploration
- **Matches versions** with the current Rust crate `Cargo.toml`, if the crate is in use; otherwise gets the most recent version
- **Accepts optional version parameter** as a semver range (same format as `Cargo.toml`) to override version selection
- **Optionally searches** within the extracted sources using regex patterns
- **Returns structured results** with file paths, line numbers, and context
- **Provides conditional responses** - only includes search results when a pattern is provided
- **Caches extractions** to avoid redundant downloads and improve performance

This eliminates the need for separate MCP servers and provides seamless integration with the existing Symposium ecosystem.

# Shiny future

> How will things will play out once this feature exists?

When developers ask the agent to work with a crate that they do not know, they will invoke the `get_rust_crate_source` MCP tool and read in the crate source. The agent will be able to give the names of specific APIs and provide accurate usage examples. Developers working in Symposium will have seamless access to Rust crate exploration:

- **Unified Interface**: Single `get_rust_crate_source` tool handles both extraction and searching
- **IDE Integration**: Results appear directly in the IDE with proper formatting and links
- **Intelligent Responses**: Tool returns only relevant fields (search results only when pattern provided)
- **Cached Performance**: Extracted crates are cached to avoid redundant downloads
- **Rich Context**: Search results include surrounding code lines for better understanding

Example workflows:
- `get_rust_crate_source(crate_name: "tokio")` → extracts and returns path info
- `get_rust_crate_source(crate_name: "tokio", pattern: "spawn")` → extracts, searches, and returns matches

# Implementation details and plan

> Tell me more about your implementation. What is your detailed implementaton plan?

## Details

### Tool parameters

The `get_rust_crate_source` tool accepts the following parameters:

```json
{
  "crate_name": "string",        // Required: Name of the crate (e.g., "tokio")
  "version": "string?",          // Optional: Semver range (e.g., "1.0", "^1.2", "~1.2.3")
  "pattern": "string?"           // Optional: Regex pattern for searching within sources
}
```

### Tool result

The response always begins with the location of the crate source:

```json
{
  "crate_name": "tokio",
  "version": "1.35.0",
  "checkout_path": "/path/to/extracted/crate",
  "message": "Crate tokio v1.35.0 extracted to /path/to/extracted/crate"
}
```

When a pattern is provided, we include two additional fields, indicating that occured in examples and matches that occurred anywhere:

```json
{
  // ... as above ...

  // Indicates the matches that occurred inside of examples.
  "example_matches": [
    {
      "file_path": "examples/hello_world.rs",
      "line_number": 8,
      "context_start_line": 6,
      "context_end_line": 10,
      "context": "#[tokio::main]\nasync fn main() {\n    tokio::spawn(async {\n        println!(\"Hello from spawn!\");\n    });"
    }
  ],

  // Indicates any other matches that occured across the codebase
  "other_matches": [
    {
      "file_path": "src/task/spawn.rs",
      "line_number": 156,
      "context_start_line": 154,
      "context_end_line": 158,
      "context": "/// Spawns a new asynchronous task\n///\npub fn spawn<T>(future: T) -> JoinHandle<T::Output>\nwhere\n    T: Future + Send + 'static,"
    }
  ],
}
```

### Crate version and location

The crate version to be fetched will be identified based on the project's lockfile, found by walking up the directory tree from the current working directory. If multiple major versions of a crate exist in the lockfile, the tool will return an error requesting the agent specify which version to use via the optional `version` parameter. When possible we'll provide the source from the existing cargo cache. If no cache is found, or the crate is not used in the project, we'll download the sources from crates.io and unpack them into a temporary directory.

The tool accepts an optional `version` parameter as a semver range (using the same format as `Cargo.toml`, e.g., "1.0", "^1.2", "~1.2.3") and will select the most recent version matching that range, just as cargo would.

## Impl phases

### Phase 1: Core Integration ✅ (Completed)
1. Copy `eg` library source into `symposium/mcp-server/src/eg/`
2. Add required dependencies to Cargo.toml
3. Implement unified `get_rust_crate_source` tool with conditional response fields
4. Fix import paths and module structure

### Phase 2: Testing and Documentation
1. Create comprehensive test suite for the tool
2. Update user documentation with usage examples
3. Create migration guide for existing standalone `eg` users
4. Performance testing and optimization

### Phase 3: Enhanced Features (Future)
1. Configurable context lines for search results
2. Search scope options (examples only vs. all source)
3. Integration with other Symposium tools for enhanced workflows
4. Smart dependency resolution (use the version that the main crate being modified depends on directly)

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## Why not use rustdoc to browse APIs?

We have observed that most developers building in Rust get good results from manually checking out the sources. This fits with the fact that agents are trained to work well from many-shot prompts, which are essentially a series of examples, and that they are trained to be able to quickly read and comprehend source code.

It is less clear that they are good at reading and navigating rustdoc source, but this is worth exploring.

## Why not use LSP to give structured information?

See previous answer -- the same logic (and desire to experiment!) applies.

## Won't checking out the full crate source waste a lot of context?

Maybe -- the impl details may not be especially relevant, but then again,when I want to work with crates, I often drill in. We might want to explore producing altered versions of the source that intentionally hide private functions and so forth, and perhaps have the agent be able to ask for additional data.

## How will we ensure the agent uses the tool?

This is indeed a good question! We will have to explore our guidance over time, both for steering the agent to use the tool and for helping it understand the OUTPUT.

## What future enhancements might we consider?

- **Smart dependency resolution**: Instead of erroring on version conflicts, use the version that the main crate being modified depends on directly
- **Workspace-aware version selection**: Handle complex workspace scenarios with multiple lockfiles
- **Integration with rust-analyzer**: Leverage LSP information for more targeted source exploration
- **Filtered source views**: Hide private implementation details to reduce context noise

# Revision history

- 2025-09-17: Initial RFD creation with completed Phase 1 implementation
