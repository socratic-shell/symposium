# Elevator pitch

> What are you proposing to change?

Add the ability for taskspaces to choose a collaborator, rather than always using the same collaboration patterns. The existing behavior becomes the "socrates" collaborator.

Add two new collaborators: `base-agent` (no collaboration patterns, base agent) and `sparkle` (based on the Sparkle MCP server).

When the `assemble_yiasou_prompt` code runs, it will use the collaborator choice to decide what to do. For `sparkle`, it will instruct the LLM to execute the sparkle tool.

Both Symposium and Sparkle MCP servers are installed as separate binaries via `cargo setup`, making all tools from both servers available to the LLM at all times. The collaborator choice only affects prompt assembly and initialization behavior.

When a new taskspace is created, users have the option to specify the collaborator, with `sparkle` being the default. The `spawn_taskspace` MCP also now has an optional parameter to specify the collaborator which defaults to the same collaborator as the current taskspace.

The `@hi` command takes an optional parameter that is the collaborator name. It defaults to the taskspace's current collaborator setting, or `sparkle` if not in a taskspace.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Currently, all Symposium taskspaces use the same collaboration patterns from `main.md` (Socratic dialogue approach). The `/yiasou` prompt assembly loads these patterns for every agent initialization, creating a one-size-fits-all collaboration style.

While the current Socratic patterns are an improvemegnt, Sparkle represents a more advanced collaboration framework with richer patterns, persistent memory, and specialized tools for partnership development.

Problems with current approach:
- Limited to basic collaboration patterns when more advanced options exist
- No access to Sparkle's advanced partnership tools and persistent memory
- No personalization of collaboration style per taskspace
- No way to experiment with different AI collaboration approaches

# What we propose to do about it

> What are you proposing to improve the situation?

Implement a collaborator system with three options:
Implement a collaborator system with three options:
- `sparkle` - Advanced collaboration patterns with persistent memory and partnership tools (the default!)
- `socrates` - Current Socratic dialogue patterns (zero-config fallback)
- `base-agent` - No collaboration patterns, base agent behavior (aliases: `claude`, `gpt`, `gemini`, `codex`)

Sparkle becomes the default because it provides strictly better collaboration patterns. Socrates remains available for the time being as a zero-config option.

Selection behavior:
- Within taskspaces: `@hi` uses the taskspace's current collaborator setting, `@hi <collaborator>` changes it and updates the taskspace via `update_taskspace`
- Outside taskspaces: `@hi` defaults to sparkle, but `@hi socrates` or `@hi claude` (or other base-agent aliases) available

Future extensibility will allow custom Sparkler names and user-defined collaborators.

Integration approach:
1. Install both Symposium and Sparkle MCP servers as separate binaries via `cargo setup`
2. Configure both servers in AI assistant MCP configurations
3. Modify taskspace data structure to store collaborator choice
4. Update `/yiasou` prompt assembly to load appropriate patterns and execute sparkle tool for sparkle collaborator
5. Implement `@hi <collaborator>` syntax for selection with taskspace persistence

# Shiny future

> How will things will play out once this feature exists?

Users can create taskspaces with different collaborators:
- `@hi sparkle` creates a taskspace with Sparkle's embodiment patterns, working memory, and partnership tools
- `@hi socrates` uses the familiar Socratic dialogue approach
- `@hi claude` (or `@hi gpt`, `@hi gemini`, etc.) provides minimal AI collaboration for focused technical work

Each taskspace maintains its collaborator choice, creating consistent collaboration experiences. Sparkle taskspaces gain access to Sparkle's advanced collaboration tools and persistent memory features.

The system becomes a platform for experimenting with different AI collaboration approaches while maintaining backward compatibility.

# Implementation details and plan

> Tell me more about your implementation. What is your detailed implementaton plan?

## Technical Architecture

**Dual MCP Servers**: Install both Symposium and Sparkle as separate MCP server binaries via `cargo setup`:
- Both servers configured in the AI assistant's MCP configuration
- All tools from both servers are always available to the LLM
- No dynamic tool routing or conditional tool exposure needed
- Simpler architecture with standard MCP server setup

**Sparkle Integration**: Install Sparkle MCP server via `cargo install --git`:
- Sparkle tools: All tools from the Sparkle MCP server (e.g., `sparkle`, `session_checkpoint`, etc.)
- Sparkle directories: `~/.sparkle/` (global patterns/insights), `.sparkle-space/` (workspace working memory)

**Prompt Assembly**: `/yiasou` prompt assembly varies by collaborator:
- `sparkle` → Load Sparkle identity files + instruct LLM to execute `sparkle` tool for initialization
- `socrates` → Load existing Socratic dialogue patterns from `socrates.md` (renamed from `main.md`)
- `base-agent` → Load minimal patterns, no special initialization

**Current System Integration**:
- Existing guidance (`symposium/mcp-server/src/guidance/main.md`) becomes "socrates" collaborator (rename to `socrates.md`)
- Add `collaborator: Option<String>` field to taskspace data structure
- Layer collaborator system on top of existing `AgentManager`/`AgentType` architecture

## Phase 1: Dual MCP Server Installation
- Update `cargo setup` to install Sparkle MCP server via `cargo install --git https://github.com/symposium-dev/sparkle.git --root sparkle-mcp`
- Add `build_and_install_sparkle_cli()` function to `setup/src/main.rs` similar to `build_and_install_rust_server()`
- Configure both Symposium and Sparkle MCP servers in AI assistant configurations
- All tools from both servers available to LLM

## Phase 2: Collaborator System
- Add `collaborator: Option<String>` to taskspace data structure
- Modify `/yiasou` prompt assembly to conditionally load:
  - `sparkle` → Sparkle identity files + execute `sparkle` tool
  - `socrates` → existing `socrates.md` (renamed from `main.md`)
  - `base-agent` → minimal patterns
- Parse `@hi <collaborator>` syntax in initial prompts, supporting aliases (`claude`, `gpt`, `gemini`, `codex` → `base-agent`)
- When `@hi <collaborator>` used in taskspace, instruct LLM to call `update_taskspace` to persist collaborator choice

## Phase 3: Tool Integration
- Ensure all Sparkle tools work properly with dual MCP server setup
- Handle Sparkle-specific directories (`~/.sparkle/`, `.sparkle-space/`)
- Verify tool functionality across different collaborator modes

## Phase 4: Crates.io Migration
- Publish `sparkle-mcp` to crates.io
- Switch from path dependency to crates.io dependency
- Remove git submodule once crate dependency is stable

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## What alternative approaches did you consider, and why did you settle on this one?

Alternative approaches considered:
1. **Separate MCP servers**: Run Sparkle and Symposium as separate MCP servers, but this would complicate tool coordination and user experience
2. **Dynamic tool routing**: Conditionally expose tools based on collaborator choice, but this adds complexity for managing tool availability
3. **Configuration files**: Store collaboration patterns in config files, but this lacks the rich tooling and state management that Sparkle provides
4. **Plugin system**: Create a general plugin architecture, but this adds complexity for a specific integration need

The embedded MCP servers approach provides the simplest integration - all tools are always available, and the collaborator choice only affects prompt assembly and initialization behavior.

## How will this affect existing taskspaces?

Existing taskspaces will continue using the current Socratic patterns by default. The `socrates` collaborator will be equivalent to current behavior, ensuring backward compatibility. All users will have access to Sparkle tools, but they'll only be used when the `sparkle` collaborator is active.

## What happens to custom Sparkler names?

The initial implementation will use the default "sparkle" name. Custom Sparkler names (like `@hi alice`) can be added in a future iteration once the basic system is working.

# Revision history

Initial version - October 8, 2025
Updated to embedded MCP servers approach - October 8, 2025
