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

### 2.5a: MCP Tool for Taskspace Updates
- **Add `update_taskspace` MCP tool**: Allow AI agents to update taskspace name and description
- **Tool parameters**: Accept name and description parameters
- **Taskspace identification**: Use PID to identify which taskspace to update
- **Metadata persistence**: Update and save `taskspace.json` file
- **UI notification**: Trigger UI refresh when taskspace metadata changes

### 2.5b: Taskspace Creation UI
- **New taskspace button**: Simple "New Taskspace" button in project view for manual creation
- **Default taskspace creation**: Create taskspace with name "Unnamed taskspace", description "TBD", and standard initial prompt
- **Directory structure creation**: Create `task-$UUID/` directory with proper naming (for both UI and MCP creation)
- **Git repository cloning**: Clone project Git URL into taskspace directory (for both UI and MCP creation)
- **Metadata generation**: Create initial `taskspace.json` with Hatchling state (for both UI and MCP creation)
- **Note**: `spawn_taskspace` MCP tool (existing) creates taskspaces with specific name/description/prompt

### 2.5c: VSCode Integration & Window Management
- **VSCode launching**: Spawn VSCode process with taskspace directory as workspace
- **Window tracking**: Use Accessibility APIs to identify and track VSCode windows
- **Window association**: Link VSCode windows to taskspace UUIDs via process tracking
- **Focus management**: Implement "bring to front" when taskspace is selected in UI
- **Process lifecycle**: Handle VSCode window creation/destruction events

### 2.5d: Visual Taskspace Display
- **Taskspace cards**: Display active taskspaces in project view with metadata
- **Screen capture system**: Periodic screenshots of VSCode windows (every 5-10 seconds)
- **Thumbnail display**: Show current taskspace state via window screenshots
- **Real-time updates**: Refresh taskspace display when windows change
- **Empty state handling**: Graceful display when no taskspaces exist

### 2.5e: Basic Taskspace Operations
- **Taskspace selection**: Click to focus associated VSCode windows
- **Taskspace status**: Visual indicators for active/inactive taskspaces
- **Taskspace metadata**: Display name, description, creation time
- **Error handling**: Graceful failures for VSCode launch issues, permission problems

## Phase 3: IPC Integration

### 3.1 Daemon Connection
- **Binary discovery**: Parse MCP configuration files to find `symposium-mcp` binary
- **Process spawning**: Launch `symposium-mcp client` as subprocess
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
3. **Phase 2.3 - NEXT**: Settings dialog with permissions and agent selection
4. **Phase 2.5**: Manual taskspace creation and VSCode integration (no MCP communication yet)
5. **Phase 3**: Establish daemon connection (can test with new MCP tools)
6. **Phase 4**: Create taskspace display UI (can use mock data initially)
7. **Phase 5**: Connect real IPC messages to UI (includes Git cloning on taskspace creation)
8. **Phase 6**: Complete remaining phases: Window management, polish, testing

## Success Criteria

The MVP is complete when:
- [x] User can create/open Symposium projects
- [ ] User can manually create new taskspaces with name/description/prompt
- [ ] VSCode launches automatically for new taskspaces
- [ ] Taskspaces appear in project view with screenshots
- [ ] User can focus taskspace windows by clicking in panel
- [ ] VSCode taskspaces launch automatically with agent tools (via MCP)
- [ ] Real-time progress logs appear in Symposium panel (via MCP)
- [ ] `spawn_taskspace` MCP tool creates new taskspaces visible in panel
- [ ] Basic window tiling works for active taskspaces

**Phase 1 Status**: ✅ COMPLETE - Users can create and open .symposium projects with native macOS UI
**Phase 2.3 Status**: 🔄 NEXT - Settings dialog with permissions and agent selection

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
## Current Status (2025-01-02)

### ✅ Completed
- **Phase 1**: Complete foundation with working project management
- **Phase 2**: MCP server tools implemented and tested
- **Phase 2.3**: Settings & permissions UI complete
- **Phase 2.4**: IPC daemon connection working, real-time status updates
- **UI Architecture**: Clean SwiftUI observation patterns, proper state management
- **Debug Infrastructure**: Centralized logging system for troubleshooting

### 🚧 In Progress - Phase 2.5: Manual Taskspace Management
- Need to implement taskspace creation UI (Phase 2.5a)
- Need to add taskspace display and management (Phase 2.5b)
- Need to test VSCode integration without MCP communication yet

### 📋 Next Steps (Phase 2.5)
1. **Taskspace Creation UI**: Add interface to manually create taskspaces
2. **Taskspace Display**: Show existing taskspaces in project view
3. **VSCode Integration**: Test opening taskspaces in VSCode
4. **Manual Management**: Basic taskspace lifecycle without daemon communication

Then move to Phase 3 for full daemon communication and orchestration.

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
