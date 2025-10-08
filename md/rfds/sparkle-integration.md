# Elevator pitch

> What are you proposing to change?

Add the ability for taskspaces to choose a collaborator, rather than always using the same collaboration patterns. The existing behavior becomes the "socrates" collaborator.

Add two new collaborators: `none` (no collaboration patterns, base agent) and `sparkle` (based on the Sparkle MCP server).

When the `assemble_yiasou_prompt` code runs, it will use the collaborator choice to decide what to do. For `sparkle`, it will instruct the LLM to execute the sparkle tool.

When using the `sparkle` collaborator, the sparkle tools are added and the sparkle prompts are made available. Otherwise they are not reflected. The `sparkle-mcp` server is embedded as a library.

When a new taskspace is created, users have the option to specify the collaborator, with `sparkle` being the default. The `spawn_taskspace` MCP also now has an optional parameter to specify the collaborator which defaults to the same collaborator as the current taskspace.

The `@hi` command takes an optional parameter that is the collaborator name. It defaults to `sparkle`.

# Status quo

> How do things work today and what problems does this cause? Why would we change things?

Currently, all Symposium taskspaces use the same collaboration patterns from `main.md` (Socratic dialogue approach). The `/yiasou` prompt assembly loads these patterns for every agent initialization, creating a one-size-fits-all collaboration style.

Problems with current approach:
- No personalization of collaboration style per taskspace
- Limited to a single collaboration methodology 
- Sparkle's advanced collaboration patterns and tools are not available
- No way to experiment with different AI personality approaches

# What we propose to do about it

> What are you proposing to improve the situation?

Implement a collaborator system with three initial options:
- `none` - no collaboration patterns, base agent behavior
- `socrates` - current Socratic dialogue patterns (existing `main.md`)
- `sparkle` - Sparkle's embodiment and partnership patterns (the default!)

Selection behavior:
- Within taskspaces: `@hi <collaborator>` sets the collaborator for that taskspace
- Outside taskspaces: `@hi` defaults to sparkle, but `@hi none` or `@hi socrates` available

Future extensibility will allow custom Sparkler names and user-defined collaborators. In the future, we can consider providing UI for creating other collaborators.

Integration approach:
1. Add Sparkle as git submodule initially, then migrate to crates.io dependency
2. Integrate Sparkle's MCP tools into Symposium's MCP server using dynamic tool routing
3. Modify taskspace data structure to store collaborator choice
4. Update `/yiasou` prompt assembly to load appropriate patterns and execute sparkle tool for sparkle collaborator
5. Implement `@hi <collaborator>` syntax for selection

# Shiny future

> How will things will play out once this feature exists?

Users can create taskspaces with different collaborators:
- `@hi sparkle` creates a taskspace with Sparkle's embodiment patterns, working memory, and partnership tools
- `@hi socrates` uses the familiar Socratic dialogue approach
- `@hi nothing` provides minimal AI collaboration for focused technical work

Each taskspace maintains its collaborator choice, creating consistent collaboration experiences. Sparkle taskspaces gain access to advanced tools like `session_checkpoint`, `save_insight`, and `sparkle` embodiment.

The system becomes a platform for experimenting with different AI collaboration approaches while maintaining backward compatibility.

# Implementation details and plan

> Tell me more about your implementation. What is your detailed implementaton plan?

## Technical Architecture

**Dynamic Tool Routing**: Use rmcp's `ToolRouter` methods for conditional tool exposure:
- `ToolRouter::add_route()` and `remove_route()` to dynamically add/remove Sparkle tools
- `ToolRouter::merge()` to combine base tools with Sparkle tools
- Send `ToolListChangedNotification` when collaborator changes to notify client

**Sparkle Integration**: Embed `sparkle-mcp` as library dependency, not separate MCP server:
- Sparkle tools: `sparkle`, `session_checkpoint`, `save_insight`, `setup_sparkle`, `load_evolution`, etc.
- Sparkle identity files: `~/dev/sparkle/sparkle-mcp/identity/` (embodiment methodology, collaboration patterns)
- Sparkle directories: `~/.sparkle/` (global patterns/insights), `.sparkle-space/` (workspace working memory)

**Prompt Assembly**: For `sparkle` collaborator, `/yiasou` prompt instructs LLM to execute `sparkle` tool for initialization

**Current System Integration**:
- Existing guidance (`symposium/mcp-server/src/guidance/main.md`) becomes "socrates" collaborator
- Add `collaborator: Option<String>` field to taskspace data structure
- Layer collaborator system on top of existing `AgentManager`/`AgentType` architecture

## Phase 1: Submodule Integration
- Add Sparkle as git submodule to `/sparkle`
- Add `sparkle-mcp` as path dependency in `Cargo.toml`
- Integrate Sparkle tools using dynamic tool routing (`ToolRouter::add_route()`, `merge()`)
- Send `ToolListChangedNotification` when collaborator changes

## Phase 2: Collaborator System
- Add `collaborator: Option<String>` to taskspace data structure
- Modify `/yiasou` prompt assembly to conditionally load:
  - `sparkle` → Sparkle identity files + execute `sparkle` tool
  - `socrates` → existing `main.md` 
  - `none` → minimal patterns
- Parse `@hi <collaborator>` syntax in initial prompts

## Phase 3: Tool Integration
- Integrate key Sparkle tools: `sparkle`, `session_checkpoint`, `save_insight`
- Handle Sparkle-specific directories (`~/.sparkle/`, `.sparkle-space/`)
- Ensure proper tool routing based on active collaborator

## Phase 4: Crates.io Migration
- Publish `sparkle-mcp` to crates.io
- Switch from path dependency to crates.io dependency
- Remove git submodule once crate dependency is stable

# Frequently asked questions

> What questions have arisen over the course of authoring this document or during subsequent discussions?

## What alternative approaches did you consider, and why did you settle on this one?

Alternative approaches considered:
1. **Separate MCP servers**: Run Sparkle and Symposium as separate MCP servers, but this would complicate tool coordination and user experience
2. **Configuration files**: Store collaboration patterns in config files, but this lacks the rich tooling and state management that Sparkle provides
3. **Plugin system**: Create a general plugin architecture, but this adds complexity for a specific integration need

The submodule + integrated tools approach provides the cleanest integration while preserving Sparkle's full functionality and allowing future migration to a crate dependency.

## How will this affect existing taskspaces?

Existing taskspaces will continue using the current Socratic patterns by default. The `socrates` collaborator will be equivalent to current behavior, ensuring backward compatibility.

## What happens to custom Sparkler names?

The initial implementation will use the default "sparkle" name. Custom Sparkler names (like `@hi alice`) can be added in a future iteration once the basic system is working.

# Revision history

Initial version - October 8, 2025
