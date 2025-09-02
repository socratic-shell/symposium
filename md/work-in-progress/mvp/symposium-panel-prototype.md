# Symposium Panel Prototype - MVP Implementation

**Status**: Planning  
**Started**: 2025-09-02  
**Goal**: Build functional taskspace orchestrator that demonstrates the core Symposium concept

## Implementation Details

Based on the [MVP README](./README.md), here are the key implementation decisions:

### Project & Taskspace Structure
- **Project location**: User selects directory via dialog when creating new project
- **Project format**: `my-project.symposium/` directories containing:
  - `project.json` with Git URL metadata
  - `task-$UUID/` subdirectories with git clones and `taskspace.json`
- **Git URL**: User input dialog, with settings to change URL later
- **Taskspace identification**: MCP server extracts UUID from directory path (hacky but functional)

### VSCode Integration  
- **Taskspace detection**: Extension checks if workspace has parent with `taskspace.json`
- **Agent launching**: Extension creates terminal and launches configured agent (Q CLI or Claude Code)
- **State management**: Hatchling → Resume state transition on first launch

### Symposium App Architecture
- **Daemon connection**: Connect immediately on startup using `symposium-mcp client`
- **Binary discovery**: Check MCP configuration of Q CLI/Claude Code (hacky but works)
- **Window management**: Use existing Accessibility APIs to bring VSCode windows to front
- **Persistence**: Update `taskspace.json` immediately on each `log_progress` call

## MVP Components to Build

### 1. Project Management
- [ ] Project selection/creation dialog
- [ ] Project settings dialog (Git URL configuration)
- [ ] Project directory structure creation

### 2. Settings & Preferences
- [ ] Accessibility permission checking
- [ ] Agent tool selection (Q CLI vs Claude Code)
- [ ] Initial setup flow

### 3. Taskspace Display
- [ ] Main panel UI showing taskspaces
- [ ] Real-time log display from `log_progress` calls
- [ ] Attention management from `signal_user` calls
- [ ] Window tiling configuration buttons

### 4. IPC Integration
- [ ] Connect to daemon via `symposium-mcp client`
- [ ] Handle `spawn_taskspace`, `log_progress`, `signal_user` messages
- [ ] Binary discovery from MCP configuration

### 5. Window Management
- [ ] Track VSCode windows by taskspace UUID
- [ ] Bring taskspace windows to front on selection
- [ ] Basic tiling layouts

## Technical Approach

1. **Phase 1**: Project management and settings dialogs
2. **Phase 2**: IPC connection and message handling  
3. **Phase 3**: Taskspace display with real data
4. **Phase 4**: Window management and tiling

## Next Steps

- [ ] Examine current Swift app structure
- [ ] Design data models for Project and Taskspace
- [ ] Create project selection/creation UI
- [ ] Implement IPC client connection

## Intentionally Hacky Implementations (To Be Revised)

The following approaches are deliberately hacky for the MVP and will need proper solutions later:

### 1. Taskspace Identification via Directory Path
**Current approach**: MCP server extracts UUID from its working directory path  
**Problem**: Fragile, assumes specific directory structure, breaks if paths change  
**Future solution**: Proper taskspace registration and ID passing through IPC messages

### 2. Binary Discovery via MCP Configuration Parsing  
**Current approach**: Symposium app reads Q CLI/Claude Code MCP configuration to find `symposium-mcp` binary location  
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
