# Symposium Panel Prototype - MVP Implementation

**Status**: Phase 1 Complete ✅  
**Started**: 2025-09-02  
**Goal**: Build functional taskspace orchestrator that demonstrates the core Symposium concept

## Phase 1 Complete - Foundation & Project Management ✅

### What's Working:
- ✅ **Data Models**: Project, Taskspace, TaskspaceLog with JSON persistence
- ✅ **Project Creation**: Users can create `.symposium` directories with name, Git URL, location selection
- ✅ **Project Opening**: Users can browse and open existing `.symposium` projects  
- ✅ **Native macOS UI**: SwiftUI interface with proper app bundle and code signing
- ✅ **Reactive State Management**: Proper view switching between project selection and project view
- ✅ **Empty State Display**: Shows "No taskspaces yet" when project has no taskspaces

### Testing:
```bash
cd symposium/macos-app
./build-app.sh
open ./.build/arm64-apple-macosx/release/Symposium.app
```

### Key Implementation Details:
- **Shared State**: Fixed ProjectManager sharing between views (was creating separate instances)
- **Thread Safety**: UI updates via `DispatchQueue.main.async` for proper SwiftUI reactivity
- **File Structure**: Projects create `project.json` with metadata, ready for `task-$UUID/` subdirectories
- **Error Handling**: Graceful error display for invalid directories, creation failures

## Next: Phase 2 - MCP Server Tool Extensions

Need to implement the missing MCP tools that agents will use:
- `spawn_taskspace` - Create new taskspaces with initial prompts
- `log_progress` - Report progress with visual indicators  
- `signal_user` - Request user attention

## Implementation Details

Based on the [MVP README](./README.md), here are the key implementation decisions:

### Project & Taskspace Structure
- **Project location**: User selects directory via dialog when creating new project ✅
- **Project format**: `my-project.symposium/` directories containing:
  - `project.json` with Git URL metadata ✅
  - `task-$UUID/` subdirectories with git clones and `taskspace.json` (Phase 2)
- **Git URL**: User input dialog, with settings to change URL later ✅
- **Taskspace identification**: MCP server extracts UUID from directory path (hacky but functional)

### VSCode Integration  
- **Taskspace detection**: Extension checks if workspace has parent with `taskspace.json`
- **Agent launching**: Extension creates terminal and launches configured agent (Q CLI or Claude Code)
- **State management**: Hatchling → Resume state transition on first launch

### Symposium App Architecture
- **Daemon connection**: Connect immediately on startup using `socratic-shell-mcp client`
- **Binary discovery**: Check MCP configuration of Q CLI/Claude Code (hacky but works)
- **Window management**: Use existing Accessibility APIs to bring VSCode windows to front
- **Persistence**: Update `taskspace.json` immediately on each `log_progress` call

## MVP Components Status

### 1. Project Management ✅
- [x] Project selection/creation dialog
- [x] Project settings dialog (Git URL configuration) - *Basic version complete*
- [x] Project directory structure creation

### 2. Settings & Preferences 🔄
- [ ] Accessibility permission checking
- [ ] Agent tool selection (Q CLI vs Claude Code)
- [ ] Initial setup flow

### 3. Taskspace Display 🔄
- [x] Main panel UI showing taskspaces (empty state)
- [ ] Real-time log display from `log_progress` calls
- [ ] Attention management from `signal_user` calls
- [ ] Window tiling configuration buttons

### 4. IPC Integration ⏳
- [ ] Connect to daemon via `socratic-shell-mcp client`
- [ ] Handle `spawn_taskspace`, `log_progress`, `signal_user` messages
- [ ] Binary discovery from MCP configuration

### 5. Window Management ⏳
- [ ] Track VSCode windows by taskspace UUID
- [ ] Bring taskspace windows to front on selection
- [ ] Basic tiling layouts

## Technical Approach

1. **Phase 1**: Project management and settings dialogs ✅
2. **Phase 2**: IPC connection and message handling  
3. **Phase 3**: Taskspace display with real data
4. **Phase 4**: Window management and tiling

## Next Steps

- [ ] Implement missing MCP server tools (`spawn_taskspace`, `log_progress`, `signal_user`)
- [ ] Define IPC message protocol for daemon communication
- [ ] Test MCP tools with simple command-line calls
- [ ] Begin Phase 3 IPC integration

## Intentionally Hacky Implementations (To Be Revised)

The following approaches are deliberately hacky for the MVP and will need proper solutions later:

### 1. Taskspace Identification via Directory Path
**Current approach**: MCP server extracts UUID from its working directory path  
**Problem**: Fragile, assumes specific directory structure, breaks if paths change  
**Future solution**: Proper taskspace registration and ID passing through IPC messages

### 2. Binary Discovery via MCP Configuration Parsing  
**Current approach**: Symposium app reads Q CLI/Claude Code MCP configuration to find `socratic-shell-mcp` binary location  
**Problem**: Brittle dependency on external tool configuration formats  
**Future solution**: Standard installation paths, proper binary discovery, or bundled binaries

### 3. VSCode Taskspace Detection via Parent Directory
**Current approach**: Extension checks if workspace has parent directory with `taskspace.json`  
**Problem**: Assumes specific directory layout, could false-positive on user projects  
**Future solution**: Explicit taskspace registration, environment variables, or workspace metadata

### 4. Window Tracking by Process Association
**Current approach**: Associate VSCode windows with taskspaces by tracking process IDs  
**Problem**: Doesn't handle multiple windows per taskspace, fragile to process lifecycle  
**Future solution**: Proper window registration system, window metadata, or IDE integration APIs

These shortcuts let us build a working demo quickly while identifying the proper architectural patterns for the production system.
