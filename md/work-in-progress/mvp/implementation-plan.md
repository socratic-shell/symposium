# MVP Implementation Plan

**Goal**: Build a working Symposium taskspace orchestrator that demonstrates the core concept through a functional macOS application.

## Phase 1: Foundation & Project Management

### 1.1 Data Models
Create Swift data structures for:
- `Project`: Contains Git URL, name, taskspaces list
- `Taskspace`: Contains UUID, name, description, state (Hatchling/Resume), logs, VSCode window reference
- `TaskspaceLog`: Progress messages with categories (info, warn, error, milestone, question)

### 1.2 Project Selection/Creation UI
- **Project picker dialog**: File browser to select existing `.symposium` directories
- **New project dialog**: Name input, Git URL input, directory selection
- **Project creation logic**: Create directory structure, generate `project.json` (no Git cloning yet)
- **Project opening**: Load existing projects, display metadata and taskspaces

### 1.3 Settings & Preferences
- **Accessibility permission checking**: Use existing permission detection code
- **Agent tool selection**: UI to choose between Q CLI and Claude Code
- **Preferences persistence**: Store user choices in UserDefaults

## Phase 2: MCP Server Tool Extensions

### 2.1 Add Missing Taskspace Tools
Extend the MCP server with the three tools needed for taskspace orchestration:

- **`spawn_taskspace`**: Create new taskspace with name, description, initial_prompt
  - Extract UUID from working directory path (hacky approach)
  - Send IPC message to Symposium app with taskspace creation request
  - Return success/failure status

- **`log_progress`**: Report progress with visual indicators
  - Parameters: message (string), category (info/warn/error/milestone/question)
  - Extract taskspace UUID from working directory
  - Send IPC message to update taskspace logs
  - Symposium app updates `taskspace.json` immediately

- **`signal_user`**: Request user attention for assistance
  - Parameters: message (string) describing why attention is needed
  - Extract taskspace UUID from working directory  
  - Send IPC message to highlight taskspace and update dock badge
  - Move taskspace to front of panel display

### 2.2 IPC Message Protocol
Define message types for daemon communication:
- `SpawnTaskspaceRequest { uuid, name, description, initial_prompt }`
- `LogProgressUpdate { taskspace_uuid, message, category, timestamp }`
- `UserSignalRequest { taskspace_uuid, message, timestamp }`

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

1. **Start with Phase 1**: Get basic project management working (create/open projects, no taskspaces yet)
2. **Add Phase 2**: Implement missing MCP server tools for taskspace orchestration
3. **Add Phase 3.1**: Establish daemon connection (can test with new MCP tools)
4. **Build Phase 4.1**: Create taskspace display UI (can use mock data initially)
5. **Integrate Phase 3.2**: Connect real IPC messages to UI (includes Git cloning on taskspace creation)
6. **Complete remaining phases**: Window management, polish, testing

## Success Criteria

The MVP is complete when:
- [ ] User can create/open Symposium projects
- [ ] VSCode taskspaces launch automatically with agent tools
- [ ] Real-time progress logs appear in Symposium panel
- [ ] User can focus taskspace windows by clicking in panel
- [ ] `spawn_taskspace` MCP tool creates new taskspaces visible in panel
- [ ] Basic window tiling works for active taskspaces

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
