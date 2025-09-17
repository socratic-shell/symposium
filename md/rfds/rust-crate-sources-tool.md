# Elevator pitch

> What are you proposing to change?

Extend Socratic Shell with a new `get_rust_sources` MCP tool that will direct LLMs to the sources from Rust crates and help to find examples of code that uses particular APIs.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

When Rust developers ask an LLM to use a crate that it does not know from its training data, LLMs typically hallucinate plausible-seeming (but in fact nonexistent) APIs. The easiest way to help an LLM get started is to find example code and/or to point it at the crate source, which currently requires manual investigation or the use of a separate (and proprietary) MCP server like context7.

## Leverage Rust's existing `examples` and rustdoc patterns

Cargo has conventions for giving example source code:

* many crates use the `examples` directory
* other crates include (tested) examples from rustdoc

Our MCP server will leverage those sources.

# What we propose to do about it

> What are you proposing to improve the situation?

Integrate Rust crate source exploration capabilities directly into the Socratic Shell MCP server through a unified `get_rust_sources` tool that:

- **Extracts crate sources** to a local cache directory for exploration
- **Matches versions** with the current Rust crate `Cargo.toml`, if the crate is in use; otherwise gets the most recent version
- **Optionally searches** within the extracted sources using regex patterns
- **Returns structured results** with file paths, line numbers, and context
- **Provides conditional responses** - only includes search results when a pattern is provided
- **Caches extractions** to avoid redundant downloads and improve performance

This eliminates the need for separate MCP servers and provides seamless integration with the existing Socratic Shell ecosystem.

# Shiny future

> How will things will play out once this feature exists?

When developers ask the LLM to work with a crate that it does not know, it will invoke the `get_rust_sources` MCP tool and read in the crate source. The agent will be able to give the names of specific APIs and . Developers working in Socratic Shell will have seamless access to Rust crate exploration:

- **Unified Interface**: Single `get_rust_sources` tool handles both extraction and searching
- **IDE Integration**: Results appear directly in the IDE with proper formatting and links
- **Intelligent Responses**: Tool returns only relevant fields (search results only when pattern provided)
- **Cached Performance**: Extracted crates are cached to avoid redundant downloads
- **Rich Context**: Search results include surrounding code lines for better understanding

Example workflows:
- `get_rust_sources(crate_name: "tokio")` → extracts and returns path info
- `get_rust_sources(crate_name: "tokio", pattern: "spawn")` → extracts, searches, and returns matches

# Implementation details and plan

> Tell me more about your implementation. What is your detailed implementaton plan?

## Details

### Crate version and location

The crate version to be fetched will be identified based on the project's lockfile. When possible we'll provide the source from the existing cargo cache. If no cache is found, or the crate is not used in the project, we'll download the sources from crates.io and unpack them into a temporary directory.

### Finding examples

When search terms are included, we will search the crate and include:

* examples, which the agent should look to with higher priority
* all matches, which may be confusing

## Impl phases

### Phase 1: Core Integration ✅ (Completed)
1. Copy `eg` library source into `socratic-shell/mcp-server/src/eg/`
2. Add required dependencies to Cargo.toml
3. Implement unified `get_rust_sources` tool with conditional response fields
4. Fix import paths and module structure

### Phase 2: Testing and Documentation
1. Create comprehensive test suite for the tool
2. Update user documentation with usage examples
3. Create migration guide for existing standalone `eg` users
4. Performance testing and optimization

### Phase 3: Enhanced Features (Future)
1. Version selection support (currently uses latest)
2. Configurable context lines for search results
3. Search scope options (examples only vs. all source)
4. Integration with other Socratic Shell tools for enhanced workflows

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## Why not use rustdoc to browse APIs?

We have observed that most developers building in Rust get good results from manually checking out the sources. This fits with the fact that LLMs are trained to work well from many-shot prompts, which are essentially a series of examples, and that they are trained to be able to quickly read and comprehend source code.

It is less clear that they are good at reading and navigating rustdoc source, but this is worth exploring.

## Why not use LSP to give structured information?

See previous answer -- the same logic (and desire to experiment!) applies.

## Won't checking out the full crate source waste a lot of context?

Maybe -- the impl details may not be especially relevant, but then again,when I want to work with crates, I often drill in. We might want to explore producing altered versions of the source that intentionally hide private functions and so forth, and perhaps have the agent be able to ask for additional data.

## How will we ensure the agent uses the tool?

This is indeed a good question! We will have to explore our guidance over time, both for steering the agent to use the tool and for helping it understand the OUTPUT.

# Revision history

- 2025-09-17: Initial RFD creation with completed Phase 1 implementation
