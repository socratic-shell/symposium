# Bringing It All Together: Monorepo Consolidation Plan

*Planning document for consolidating socratic-shell ecosystem repositories into this one, beginning with `socratic-shell` and `dialectic`*

## Goal

A single repository (`symposium`, this repository) that contains

* user-wide prompts
* the Symposium OS X application
* the Symposium VSCode extension 
* the Symposium MCP server offering various tools and which can be launched as a daemon
* a `mdbook` that includes
  * user's guide
    * installation instructions
  * implementation details
    * architecture overview
    * walkthroughs
  * reference material
    * individual `md` files with the results of deep research
* convenient commands via Rust's xtask pattern, such that `cargo setup` installs the
  * VSCode extension
  * MCP server
  * required prompts
* a "new project template"

## Functionality that will be provided

* the OS X application, when started, will be the "mastermind" that provides
  * automated setup, integrating and configuring the user's experience
  * the ability to move between "agentspaces", which combine
    * a VSCode instance (later extended to add'l IDEs) running the Symposium extension
    * a Claude Code instance running the Symposium MCP server and configured with Symposium context
* the Symposium MCP server will
  * allow the user to easily create new agentspaces and track progress
  * provide code walkthroughs and code editing tools that integrate with the IDE
  * allow the agent to easily access the source of dependencies and to find examples to guide its code generation
* telemetry and monitoring
  * we will monitor the users' chats to assess how well our setup is working
  * anonymous feedback will be gathered and collected

## Implementation overview and architecture

* the OS X application (implemented in swift)
  * launches new agentspaces, updates the log
  * configures setup (e.g., requesting accessibility perms, installing the IDE extension, running `claude mcp add`, etc)
  * hosts an IPC channel
    * receives messages over an IPC bus and broadcasts them to all subscribers (itself, IDE extension, MCP servers)
    * in some cases, acts on the messages received
* the IDE extension (implemented in typescript)
  * on startup, ensures the AI agent is runningin an internal terminal
  * presents code walkthroughs in a sidebar
  * enables the user to embed `<symposium-ref uuid="..."/>` "references" in their chat, which identify pieces of code or allow them to ask questions
* the MCP server (implemented in Rust) offers tools that
  * spawn new agentspaces and log progress (by sending IPC messages to the OS X application)
  * retrieve the content of `symposium-ref`
  * present code walkthroughs to the user
  * let the agent leverage IDE capabilities (e.g., "find all references") by sending IPC messages to the extension

## Points of future extensibility

In order of precedence, we expect to have:

* Support for more coding agents (not just Claude Code)
* Support for more IDEs (not just VSCode)
* More MCP functionality
  * ability to find dependency examples
  * memory server
* Support for Windows and Linux users

This implies that we should centralize as much of the functionality into the MCP server as possible, as it is portable across all of these environments. The IDE extensions in particular strive to be dumb.

## Existing components and how we'll integrate them

The `socratic-shell` github org has numerous repositories that already contain most of the content we want to have. Our task is to pull in those contents to populate this repository and combine them into a coherent system. The purpose of this document is to plan how we will do that.

* user-wide prompts
  * these are found in `socratic-shell` github repository
* the Symposium OS X application
  * a skeleton for this already exists in this repository
* the Symposium VSCode extension 
  * the `dialectic` repository contains a VSCode extension that provides walkthroughs and "XML reference" functionality
* the Symposium MCP server combines
  * the `dialectic` repository contains a VSCode extension

The repositories are checked out locally already:

* `socratic-shell` -- `~/dev/socratic-shell`
* `dialectic` -- `~/dev/dialectic`

## Implementation Plan

Based on analysis of the dialectic repository structure, we have a sophisticated system with:
- Rust MCP server with daemon capabilities (`dialectic-mcp-server`)
- VSCode extension with walkthroughs and synthetic PRs (`dialectic`)
- Extensive documentation and research (mdbook)
- Unix socket-based IPC architecture

### Phase 0: Physical Consolidation ✅
**Status: COMPLETE** - Dialectic repository contents copied to `/Users/nikomat/dev/symposium/dialectic-repo/`

### Phase 1: Gentle Integration
**Goal**: Get dialectic working within symposium without breaking existing functionality

**Approach**: Establish "symposium as orchestrator" pattern while keeping dialectic's architecture intact

**TODO**: Resolve key integration questions:

#### Directory Structure Decision

The symposium repository should have the following directories

* `application/osx` (contains Symposium app, comes directly from this repository to start)
* `ide/vscode` (contains the VSCode extension, comes directly from `dialectic` repository to start)
* `mcp-server` (contains the MCP server, comes directly from `dialectic` repository to start)
* `prompts` (contains user prompt)
* `md` (contains the contents of the mdbook, merged contents of `md` and `dialectic-repo/md`)

#### Build System Integration

Generalize Dialectic's existing `cargo setup` comamnd to serve as a general purpose builder interface for Symposium.

#### Daemon Lifecycle Management

Let's keep the existing daemon approach. The daemon is the same binary as the MCP server and can be started by "anyone" (the OS X app, the extension, the MCP servers). The goal is that even if the user is not using the Symposium app things will generally work fine, it's just that new agentspace functionality will be disabled.

#### Extension Identity

Let's keep it as Dialectic initially but very soon after change to Symposium.

My main goal is to move in as small increments as possible.

#### Phase 1: Detailed Implementation Steps

**Step 1.1: Directory Restructure** ✅
- [x] Create new directory structure in symposium:
  - [x] `mkdir -p application/osx ide/vscode mcp-server prompts`
  - [x] Move existing symposium OSX app content to `application/osx/`
  - [x] Copy dialectic extension to `ide/vscode/`
  - [x] Copy dialectic server to `mcp-server/`
  - [x] Merge mdbook content: `md/` + `dialectic-repo/md/` → `md/`

**Test 1.1**: Directory structure matches plan, no files lost
- [ ] All expected directories exist
- [ ] Original symposium OSX content preserved in `application/osx/`
- [ ] Dialectic extension present in `ide/vscode/`
- [ ] Dialectic server present in `mcp-server/`
- [ ] Combined mdbook builds: `mdbook serve md/`

**Step 1.2: Update Workspace Configuration** ✅
- [x] Update root `Cargo.toml` to reference new `mcp-server/` path
- [x] Update paths in `mcp-server/Cargo.toml` if needed  
- [x] Update VSCode extension paths in `ide/vscode/package.json`
- [x] Update mdbook paths in `book.toml`

**Test 1.2**: Build system works from new locations
- [ ] `cargo check` passes from repository root
- [ ] `cargo build` succeeds for mcp-server from `mcp-server/`
- [ ] VSCode extension builds: `cd ide/vscode && npm run compile`
- [ ] mdbook builds: `mdbook build md/`

**Step 1.3: Rename from Dialectic to Symposium**
- [ ] Rename binary: `dialectic-mcp-server` → `symposium-mcp` in `mcp-server/Cargo.toml`
- [ ] Update extension identity in `ide/vscode/package.json`:
  - [ ] `name`: `dialectic` → `symposium`
  - [ ] `displayName`: `Dialectic Walkthroughs` → `Symposium`
  - [ ] `publisher`: update as needed
  - [ ] Commands: `dialectic.*` → `symposium.*`
  - [ ] Views: `dialectic` → `symposium` 
- [ ] Update extension TypeScript code to use new command names
- [ ] Update MCP server references from "dialectic" to "symposium"
- [ ] Update socket file paths and other runtime identifiers
- [ ] Update documentation and comments throughout codebase

**Step 1.4: Adapt Setup Command**
- [ ] Copy dialectic's `setup/` directory to symposium root
- [ ] Update setup tool to work with new directory structure
- [ ] Update setup tool to install as "symposium" everywhere (no "dialectic" references)
- [ ] Update setup tool to reference new binary name `symposium-mcp`
- [ ] Test setup tool builds: `cargo build -p setup`

**Test 1.3**: Renaming is complete and consistent
- [ ] No "dialectic" references remain in code, configs, or documentation
- [ ] Binary is named `symposium-mcp` and builds successfully
- [ ] Extension identifies as "symposium" in VSCode
- [ ] All commands use `symposium.*` prefix
- [ ] Socket paths and runtime IDs use "symposium"

**Test 1.4**: Setup command works with new names and structure  
- [ ] `cargo setup --dev` builds all components successfully
- [ ] Extension installs as "symposium" (check with `code --list-extensions`)
- [ ] MCP server binary exists as `symposium-mcp` in expected location
- [ ] Setup creates correct configuration files with symposium naming

**Step 1.5: Integration Testing**
- [ ] Test full symposium workflow in new structure:
  - [ ] Start daemon: `./target/debug/symposium-mcp --daemon`
  - [ ] Verify VSCode extension activates and connects to daemon
  - [ ] Test basic MCP functionality with Claude Code
  - [ ] Test walkthrough presentation in VSCode
  - [ ] Test synthetic PR creation and display

**Test 1.5**: End-to-end symposium functionality works
- [ ] Daemon starts and creates socket file (with symposium naming)
- [ ] VSCode extension shows "Symposium" in activity bar (not "Socratic Shell")
- [ ] Claude Code can connect to symposium MCP server
- [ ] `present_walkthrough` tool works and displays in VSCode
- [ ] `request_review` tool creates synthetic PRs
- [ ] File navigation from walkthroughs works
- [ ] Comment threads display correctly in synthetic PRs

**Step 1.6: Clean Up and Validation**
- [ ] Remove `dialectic-repo/` directory (original copy)
- [ ] Final search for any remaining "dialectic" references and replace with "symposium"
- [ ] Update documentation to reflect new structure and naming
- [ ] Test clean build from scratch: `git clean -fdx && cargo setup --dev`

**Test 1.6**: System is self-contained and reproducible with symposium identity
- [ ] Fresh clone + `cargo setup --dev` works completely
- [ ] All tests from previous steps still pass
- [ ] Zero "dialectic" references anywhere in codebase
- [ ] No references to old `dialectic-repo/` directory
- [ ] Documentation reflects new structure and symposium branding
- [ ] MCP server identifies as "symposium-mcp" in all contexts

#### Phase 1 Success Criteria
At the end of Phase 1, the user should be able to:
- [ ] Run `cargo setup --dev` and get a fully working symposium system
- [ ] Use all symposium features (walkthroughs, synthetic PRs, etc.) - functionality identical to original dialectic
- [ ] Everything works from new directory structure with complete symposium branding
- [ ] Zero "dialectic" references remain anywhere in the system
- [ ] Ready to begin Phase 2 (prompt integration) from stable symposium foundation

### Phase 2: Prompt Consolidation
**Goal**: Integrate socratic-shell prompts into symposium structure

**Dependencies**: Phase 1 complete, directory structure established

**Tasks**:
- Copy socratic-shell prompts to symposium
- Update prompt references/paths
- Integrate with symposium's configuration system

### Phase 3: True Architectural Unification
**Goal**: Reshape into the final vision from the architecture overview

**Key Decisions**:
- Daemon-in-MCP-server vs daemon-in-OSX-app
- Full IPC bus architecture implementation
- Extension becomes "dumb" as intended
- Unified build and setup system

### Phase 4: Extended Integration
**Goal**: Add remaining functionality from the vision

**Tasks**:
- Agentspace management through OSX app
- Telemetry and monitoring systems
- New project template system
- Dependency example finding
- Memory server functionality

## Current Status

**Ready for Phase 1**: Waiting for user decisions on integration approach questions above.

