# MVP Implementation Plan

**Goal**: Build a working Symposium taskspace orchestrator that demonstrates the core concept through a functional macOS application.

## Phase 1: Foundation & Project Management ✅

### 1.1 Data Models ✅
Create Swift data structures for:
- ✅ `Project`: Contains Git URL, name, taskspaces list
- ✅ `Taskspace`: Contains UUID, name, description, state (Hatchling/Resume), logs, VSCode window reference
- ✅ `TaskspaceLog`: Progress messages with categories (info, warn, error, milestone, question)

### 1.2 Project Selection/Creation UI ✅
- ✅ **Project picker dialog**: File browser to select existing `.symposium` directories
- ✅ **New project dialog**: Name input, Git URL input, directory selection
- ✅ **Project creation logic**: Create directory structure, generate `project.json` (no Git cloning yet)
- ✅ **Project opening**: Load existing projects, display metadata and taskspaces
- ✅ **Fixed SwiftUI observation**: ProjectView properly observes DaemonManager for real-time UI updates
- ✅ **Clean UI states**: Loading → Connecting to daemon → Full project interface

### 1.3 Settings & Preferences ✅
- ✅ **Accessibility permission checking**: Use existing permission detection code
- ✅ **Agent tool selection**: UI to choose between Q CLI and Claude Code
- ✅ **Preferences persistence**: Store user choices in UserDefaults

### 1.4 IPC Daemon Integration ✅
- ✅ **Daemon connection**: Fixed client stdin pipe issue, daemon now connects properly
- ✅ **Connection status**: Real-time daemon connection status in UI
- ✅ **Debug logging**: Centralized Logger system for troubleshooting
- ✅ **Clean architecture**: Separated ProjectView into own file, proper state management

## Phase 2: MCP Server Tool Extensions ✅

### 2.1 Add Missing Taskspace Tools ✅
Extend the MCP server with the three tools needed for taskspace orchestration:

- ✅ **`spawn_taskspace`**: Create new taskspace with name, description, initial_prompt
  - ✅ Extract project path and UUID from working directory path using robust traversal
  - ✅ Send IPC message to Symposium app with taskspace creation request
  - ✅ Return success/failure status

- ✅ **`log_progress`**: Report progress with visual indicators
  - ✅ Parameters: message (string), category (info/warn/error/milestone/question or emojis)
  - ✅ Extract taskspace UUID and project path from working directory
  - ✅ Send IPC message to update taskspace logs
  - ✅ Symposium app updates `taskspace.json` immediately

- ✅ **`signal_user`**: Request user attention for assistance
  - ✅ Parameters: message (string) describing why attention is needed
  - ✅ Extract taskspace UUID and project path from working directory  
  - ✅ Send IPC message to highlight taskspace and update dock badge
  - ✅ Move taskspace to front of panel display

### 2.2 IPC Message Protocol ✅
Define message types for daemon communication:
- ✅ `SpawnTaskspacePayload { project_path, taskspace_uuid, name, task_description, initial_prompt }`
- ✅ `LogProgressPayload { project_path, taskspace_uuid, message, category }`
- ✅ `SignalUserPayload { project_path, taskspace_uuid, message }`
- ✅ `DeleteTaskspacePayload { project_path, taskspace_uuid }`

**Implementation completed**:
- ✅ Added comprehensive integration tests
- ✅ Updated documentation with anchor-based includes
- ✅ Robust directory structure parsing
- ✅ Support for both text and emoji category formats

## Phase 2.3: Settings Dialog & Permissions

### 2.3a: Permission Management
- **Accessibility permission checking**: Detect current accessibility permissions status
- **Screen recording permission checking**: Detect screen capture permissions (required for screenshots)
- **Permission request UI**: Guide user through granting required permissions in System Preferences
- **Permission status display**: Visual indicators (✅ granted, ❌ denied, ⚠️ required)
- **Debug options**: Reset permissions button for testing (`tccutil reset`)

### 2.3b: Agent Tool Selection
- **Agent detection**: Scan system for installed Q CLI and Claude Code
- **Agent selection UI**: Radio buttons or dropdown to choose preferred agent
- **Agent validation**: Verify selected agent is properly configured with MCP
- **Preferences persistence**: Store agent choice in UserDefaults

### 2.3c: Settings Dialog Integration
- **Settings window**: Dedicated settings dialog accessible from main menu
- **Startup flow**: Show settings on first launch or when permissions missing
- **Settings validation**: Prevent proceeding without required permissions
- **Help text**: Clear instructions for each permission type and why it's needed

## Phase 2.5: Manual Taskspace Management

### 2.5a: MCP Tool for Taskspace Updates ✅
- **Add `update_taskspace` MCP tool**: ✅ Allow AI agents to update taskspace name and description
- **Tool parameters**: ✅ Accept name and description parameters
- **Taskspace identification**: ✅ Detect taskspace from current working directory (same as other MCP tools)
- **IPC message**: ✅ Send update message to daemon (similar to `log_progress`, `signal_user`)
- **App-side handling**: ✅ Symposium app receives message, updates taskspace in memory, and refreshes UI

### 2.5b: Taskspace Creation UI ✅
- **New taskspace button**: ✅ Simple "New Taskspace" button in project view for manual creation
- **Default taskspace creation**: ✅ Create taskspace with name "Unnamed taskspace", description "TBD", and standard initial prompt
- **Directory structure creation**: ✅ Create `task-$UUID/` directory with proper naming (for both UI and MCP creation)
- **Git repository cloning**: ✅ Clone project Git URL into taskspace directory (for both UI and MCP creation)
- **Metadata generation**: ✅ Create initial `taskspace.json` with Hatchling state (for both UI and MCP creation)
- **Note**: `spawn_taskspace` MCP tool (existing) creates taskspaces with specific name/description/prompt

### 2.5c: VSCode Extension Taskspace Detection & Agent Launch ✅
- **Taskspace detection**: ✅ Check if VSCode is running in `task-$UUID` directory with `../taskspace.json`
- **Agent auto-launch**: ✅ Create terminal and start configured AI agent (Q CLI or Claude Code)
- **Initial prompt handling**: ✅ If taskspace is Hatchling state, send initial prompt to agent
- **State transition**: ✅ Send IPC message to change taskspace state from Hatchling to Resume
- **Window registration**: ✅ Register VSCode window with Symposium app for this taskspace

### 2.5d: VSCode Integration & Window Management
- **VSCode launching**: Spawn VSCode process with taskspace directory as workspace (from app side)
- **Window tracking**: Use Accessibility APIs to identify and track VSCode windows
- **Window association**: Link VSCode windows to taskspace UUIDs via process tracking
- **Focus management**: Implement "bring to front" when taskspace is selected in UI
- **Process lifecycle**: Handle VSCode window creation/destruction events

### 2.5e: Visual Taskspace Display
- **Taskspace cards**: Display active taskspaces in project view with metadata
- **Screen capture system**: Periodic screenshots of VSCode windows (every 5-10 seconds)
- **Thumbnail display**: Show current taskspace state via window screenshots
- **Real-time updates**: Refresh taskspace display when windows change
- **Empty state handling**: Graceful display when no taskspaces exist

### 2.5f: Basic Taskspace Operations
- **Taskspace selection**: Click to focus associated VSCode windows
- **Taskspace status**: Visual indicators for active/inactive taskspaces
- **Taskspace metadata**: Display name, description, creation time
- **Error handling**: Graceful failures for VSCode launch issues, permission problems

### 2.5g: Project Lifecycle Management
- **Project opening**: When opening a project, launch VSCode for all existing taskspaces
- **Project closing**: When closing/switching projects, close VSCode windows for all taskspaces
- **Window coordination**: Track which VSCode windows belong to which project
- **Clean shutdown**: Ensure no orphaned VSCode windows when switching between projects
- **State preservation**: Maintain taskspace state across project open/close cycles

## Phase 3: IPC Integration

### 3.1 Daemon Connection
- **Binary discovery**: Parse MCP configuration files to find `socratic-shell-mcp` binary
- **Process spawning**: Launch `socratic-shell-mcp client` as subprocess
- **Communication setup**: stdin/stdout pipes for JSON message exchange

### 3.2 Message Handling
- **Incoming messages**: Handle `spawn_taskspace`, `log_progress`, `signal_user`
- **Taskspace creation**: Create `task-$UUID/` directory, clone Git repo, generate `taskspace.json`
- **Taskspace tracking**: Map messages to taskspaces via UUID extraction
- **State persistence**: Update `taskspace.json` files immediately on changes

### 3.3 VSCode Process Tracking
- **Window enumeration**: Use existing Accessibility APIs to find VSCode windows
- **Process association**: Link VSCode processes to taskspace UUIDs
- **Window state tracking**: Monitor window creation/destruction

## Phase 4: Main Panel UI

### 4.1 Taskspace Display
- **Taskspace list view**: ScrollView with individual taskspace cards
- **Log display**: Real-time progress messages with visual indicators
- **Attention management**: Highlight taskspaces requesting user attention
- **Empty state**: UI for when no taskspaces exist

### 4.2 Tiling Configuration
- **Layout buttons**: Top toolbar with common window arrangements
- **Window positioning**: Use Accessibility APIs to arrange VSCode windows
- **Tiling mode**: Monitor and maintain window proportions during resize

### 4.3 Taskspace Actions
- **Selection handling**: Bring taskspace windows to front on click
- **New taskspace creation**: UI to spawn new taskspaces with initial prompts
- **Taskspace management**: Basic operations (focus, close, etc.)

## Phase 5: Integration & Polish

### 5.1 End-to-End Testing
- **Full workflow validation**: Project creation → taskspace spawning → progress tracking
- **Error handling**: Graceful failures for missing binaries, permission issues
- **Performance testing**: Multiple taskspaces, rapid message handling

### 5.2 User Experience
- **Dock integration**: Badge counts for active taskspaces and attention requests
- **Panel behavior**: Show/hide on dock icon click
- **Visual polish**: Consistent styling, loading states, animations

### 5.3 Documentation & Deployment
- **Installation instructions**: Update setup process for new app
- **User guide**: Basic usage documentation
- **Known limitations**: Document hacky implementations and future improvements

## Implementation Order

1. **Phase 1 - COMPLETE ✅**: Get basic project management working (create/open projects, no taskspaces yet)
2. **Phase 2 - COMPLETE ✅**: Implement missing MCP server tools for taskspace orchestration
3. **Phase 2.3 - COMPLETE ✅**: Settings dialog with permissions and agent selection
4. **Phase 2.5 - COMPLETE ✅**: Manual taskspace creation and VSCode integration
5. **Phase 2.7 - COMPLETE ✅**: Activity logs display with real-time updates
6. **Phase 2.8 - Window Registration - COMPLETE ✅**: Window registration system for reliable window-taskspace association
7. **Phase 2.9 - Window Screenshots - NEXT**: Visual taskspace previews using captured window screenshots
8. **Phase 2.10 - Window Focus**: Click taskspace to bring associated windows to front
9. **Phase 2.11 - Window Tiling**: Optional tile mode with taskspace preview anchored left, IDE window taking remaining space
10. **Phase 2.12 - Window Cleanup**: Close associated IDE windows when app closes
11. **Phase 3 - Enhanced Features**: State indicators, lifecycle management, dock integration
12. **Phase 4 - Advanced UI**: Enhanced logs, multi-project support, visual polish
13. **Phase 5 - Integration Testing**: Integration testing and deployment preparation

## Success Criteria

The MVP is complete when:
- [x] User can create/open Symposium projects
- [x] User can manually create new taskspaces with name/description/prompt
- [x] VSCode launches automatically for new taskspaces
- [x] Taskspaces appear in project view with screenshots
- [ ] User can focus taskspace windows by clicking in panel
- [x] VSCode taskspaces launch automatically with agent tools (via MCP)
- [x] Real-time progress logs appear in Symposium panel (via MCP)
- [ ] `spawn_taskspace` MCP tool creates new taskspaces visible in panel
- [ ] Basic window tiling works for active taskspaces

**Phase 1 Status**: ✅ COMPLETE - Users can create and open .symposium projects with native macOS UI
**Phase 2.7 Status**: ✅ COMPLETE - Activity logs now display in real-time via fixed UUID case sensitivity

## Technical Risks & Mitigations

**Risk**: Binary discovery fails  
**Mitigation**: Fallback to PATH lookup, manual configuration option

**Risk**: VSCode window tracking breaks  
**Mitigation**: Graceful degradation, manual window association

**Risk**: IPC message parsing errors  
**Mitigation**: Robust error handling, message validation, logging

**Risk**: Accessibility API limitations  
**Mitigation**: Test early, document limitations, provide manual alternatives

## Intentionally Hacky Implementations (To Be Revised)

The following approaches are deliberately hacky for the MVP and will need proper solutions later:

### 1. Taskspace Identification via Directory Path
## Current Status (2025-09-04)

### ✅ Completed
- **Phase 1**: Complete foundation with working project management
- **Phase 2**: MCP server tools implemented and tested
- **Phase 2.3**: Settings & permissions UI complete
- **Phase 2.4**: IPC daemon connection working, real-time status updates
- **UI Architecture**: Clean SwiftUI observation patterns, proper state management
- **Debug Infrastructure**: Centralized logging system for troubleshooting

### ✅ Completed - Phase 2.5: Manual Taskspace Management
- **Taskspace creation UI**: ✅ Manual taskspace creation with "New Taskspace" button
- **Agent auto-launch**: ✅ VSCode extension detects taskspaces and launches AI agents
- **Taskspace updates**: ✅ AI agents can update taskspace name/description via MCP tools
- **UI updates**: ✅ Symposium app reflects taskspace changes in real-time
- **Auto-launch on load**: ✅ VSCode windows open automatically for active taskspaces when loading projects
- **State transitions**: ✅ Taskspaces automatically transition from Hatchling to Resume on first agent activity
- **Persistence**: ✅ All taskspace changes (name, description, state) persist to disk in taskspace.json
- **MCP communication**: ✅ Fixed JSON payload parsing for snake_case/camelCase mismatch
- **VSCode launching**: ✅ Improved to use 'code' command with fallback, separate windows per taskspace

### ✅ Completed - Phase 2.6: Core Bug Fixes & Polish
- **JSON payload parsing**: ✅ Fixed snake_case/camelCase mismatch between MCP server and Swift app
- **State transition timing**: ✅ Fixed Hatchling→Resume transitions to happen in same update cycle
- **Taskspace persistence**: ✅ Added missing save() calls to persist update_taskspace changes
- **VSCode window isolation**: ✅ Each taskspace opens in separate VSCode window using 'code' command
- **Error handling**: ✅ Proper error handling and logging for all IPC message types

### ✅ Completed - Phase 2.7: Activity Logs Display
- **UUID case sensitivity fix**: ✅ Fixed case-insensitive UUID comparisons for taskspace lookup
- **Activity logs UI**: ✅ TaskspaceCard already displays last 3 logs with icons and messages
- **Real-time log updates**: ✅ log_progress MCP tool now works correctly with UI updates
- **Visual indicators**: ✅ Emoji icons for different log categories (info, warn, error, milestone, question)

### ✅ Completed - Phase 2.8: Window Registration System
- **IPC protocol foundation**: ✅ Added TaskspaceRollCall and RegisterTaskspaceWindow message types across all codebases
- **VSCode extension implementation**: ✅ Complete window registration flow with title handshake and roll-call handling
- **Swift app broadcasting**: ✅ Re-register Windows button sends taskspace_roll_call for all taskspaces
- **Swift message handling**: ✅ Basic register_taskspace_window message parsing and response structure
- **Window scanning logic**: ✅ Implemented findWindowBySubstring() using CGWindowListCopyWindowInfo with substring matching
- **Taskspace-window association storage**: ✅ Added [UUID: CGWindowID] dictionary in ProjectManager with validation
- **Manual registration testing**: ✅ Complete registration flow verified working with successful window associations
- **Automatic registration on extension startup**: ✅ Extension auto-registers when it starts up if in taskspace

### ✅ Completed - Phase 2.9: Window-Per-Project Architecture
- **Multi-window architecture**: ✅ Separate splash window for setup, dedicated project windows for each project
- **Window titles**: ✅ Project windows show project name in title bar instead of generic "Symposium"
- **Smart sidebar sizing**: ✅ Project windows default to 1/3 screen width (300-500px), most of screen height, but remain manually resizable
- **Window positioning**: ✅ Project windows position themselves as left-side panels with proper margins
- **UI cleanup**: ✅ Removed redundant "Close Project" button since users can close windows directly
- **Window lifecycle**: ✅ Each project gets independent window, splash closes when project opens
- **SwiftUI window management**: ✅ Fixed openWindow issues, proper window group definitions with type-safe parameters

### ✅ Completed - Phase 2.10: Window Screenshots - Visual Taskspace Previews
- ✅ **Core screenshot system**: ScreenshotManager using ScreenCaptureKit (macOS 14.0+)
- ✅ **Screenshot triggers**: Capture on window registration and log updates  
- ✅ **TaskspaceCard UI integration**: Display screenshots when available, fallback to placeholders
- ✅ **Permission integration**: Works with existing PermissionManager for Screen Recording
- ✅ **Memory management**: Screenshot caching with cleanup functionality
- ✅ **UI observation issues**: Fixed SwiftUI observation patterns and timing issues
- ✅ **Screenshot architecture**: Simplified and debugged screenshot capture system

**Implementation Complete:**
- Full screenshot capture system implemented (commits 17d6e80, 080a2a9, 1578c6c, 91c234c)
- Window registration successfully associating taskspaces with CGWindowIDs
- Screenshots now properly display in TaskspaceCard UI
- Fixed UI refresh issues through improved SwiftUI observation patterns
- Added comprehensive debug logging for troubleshooting

**Success Criteria:**
- ✅ Connected taskspaces show live screenshots of their VSCode windows
- ✅ Disconnected taskspaces show clear reload indicator
- ✅ UI remains responsive during screenshot operations (async implementation)
- ✅ Screenshots update appropriately when windows change

### 📋 Phase 2.11 - Window Focus: Click to Bring Windows Forward
- **TaskspaceCard click handler**: Add click action to bring associated windows to front
- **Window focus implementation**: Use CGWindowID to focus/raise VSCode windows via CGS APIs
- **Multi-window support**: Handle cases where taskspace has multiple associated windows
- **Error handling**: Graceful fallback when window no longer exists or focus fails
- **Visual feedback**: Show loading/focusing state during window operations

### 📋 Phase 2.12 - Window Tiling: Optional Tile Mode
- **Tile mode toggle**: Add UI control to enable/disable tiling for taskspaces
- **Layout calculation**: Position taskspace preview on left, IDE window taking remaining screen space
- **Window positioning**: Use existing CGS window management APIs for precise positioning
- **Screen awareness**: Handle multiple monitors and screen resolution changes
- **Persistence**: Remember tile mode preferences per taskspace

### 📋 Phase 2.13 - Window Cleanup: Close IDE Windows on App Exit
- **App termination handler**: Detect when Symposium app is closing
- **Associated window cleanup**: Close all registered VSCode windows before app exit
- **Graceful shutdown**: Allow VSCode to save work before forced closure
- **User preference**: Optional setting to disable automatic window cleanup

### 📋 Phase 3 - Enhanced Features: Advanced Functionality
1. **Enhanced Activity Logs**: Expand log display, add timestamps, log history view
2. **State Indicators**: Visual indicators for taskspace states (Hatchling, Resume, etc.)
3. **Taskspace Lifecycle**: Add archived/paused/completed states
4. **Dock Integration**: Badge counts and notifications for user attention requests
5. **Multi-project Support**: Handle multiple projects simultaneously

### 🧹 Future Cleanup Items
- **TaskspaceId wrapper type**: Replace `UUID` with `TaskspaceId` wrapper for better type safety and clear intent in function signatures
- **Window association persistence**: Consider persisting taskspace-window associations across app restarts
- **Generalize Re-register Windows to Reload button**: Expand functionality to launch VSCode/agents for taskspaces that aren't running, not just re-register existing windows

### 🎨 Polish Items for Later
- **Menu Items for Window Management**:
  - File menu enhancements: Add "Open Project..." and "New Project..." menu items
  - Window menu: Add menu items to switch between open project windows
  - Project management: Menu items for common project operations
  - Keyboard shortcuts: Assign standard shortcuts for project operations (⌘O for open, ⌘N for new, etc.)
  - Menu items properly enabled/disabled based on app state

Then move to Phase 3 for full daemon communication and orchestration.

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
