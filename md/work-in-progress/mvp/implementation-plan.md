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
- **Project creation logic**: Create directory structure, clone Git repo, generate `project.json`

### 1.3 Settings & Preferences
- **Accessibility permission checking**: Use existing permission detection code
- **Agent tool selection**: UI to choose between Q CLI and Claude Code
- **Preferences persistence**: Store user choices in UserDefaults

## Phase 2: IPC Integration

### 2.1 Daemon Connection
- **Binary discovery**: Parse MCP configuration files to find `symposium-mcp` binary
- **Process spawning**: Launch `symposium-mcp client` as subprocess
- **Communication setup**: stdin/stdout pipes for JSON message exchange

### 2.2 Message Handling
- **Incoming messages**: Handle `spawn_taskspace`, `log_progress`, `signal_user`
- **Taskspace tracking**: Map messages to taskspaces via UUID extraction
- **State persistence**: Update `taskspace.json` files immediately on changes

### 2.3 VSCode Process Tracking
- **Window enumeration**: Use existing Accessibility APIs to find VSCode windows
- **Process association**: Link VSCode processes to taskspace UUIDs
- **Window state tracking**: Monitor window creation/destruction

## Phase 3: Main Panel UI

### 3.1 Taskspace Display
- **Taskspace list view**: ScrollView with individual taskspace cards
- **Log display**: Real-time progress messages with visual indicators
- **Attention management**: Highlight taskspaces requesting user attention
- **Empty state**: UI for when no taskspaces exist

### 3.2 Tiling Configuration
- **Layout buttons**: Top toolbar with common window arrangements
- **Window positioning**: Use Accessibility APIs to arrange VSCode windows
- **Tiling mode**: Monitor and maintain window proportions during resize

### 3.3 Taskspace Actions
- **Selection handling**: Bring taskspace windows to front on click
- **New taskspace creation**: UI to spawn new taskspaces with initial prompts
- **Taskspace management**: Basic operations (focus, close, etc.)

## Phase 4: Integration & Polish

### 4.1 End-to-End Testing
- **Full workflow validation**: Project creation → taskspace spawning → progress tracking
- **Error handling**: Graceful failures for missing binaries, permission issues
- **Performance testing**: Multiple taskspaces, rapid message handling

### 4.2 User Experience
- **Dock integration**: Badge counts for active taskspaces and attention requests
- **Panel behavior**: Show/hide on dock icon click
- **Visual polish**: Consistent styling, loading states, animations

### 4.3 Documentation & Deployment
- **Installation instructions**: Update setup process for new app
- **User guide**: Basic usage documentation
- **Known limitations**: Document hacky implementations and future improvements

## Implementation Order

1. **Start with Phase 1.1-1.2**: Get basic project management working
2. **Add Phase 2.1**: Establish daemon connection (can test with mock messages)
3. **Build Phase 3.1**: Create taskspace display UI (can use mock data initially)
4. **Integrate Phase 2.2**: Connect real IPC messages to UI
5. **Complete remaining phases**: Window management, polish, testing

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
