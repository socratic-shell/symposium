# Dock-Activated Interface Implementation Plan

*Step-by-step transition from multi-window to dock-activated interface*

## Overview

This document outlines the implementation plan for transitioning Symposium from its current multi-window architecture to the new dock-activated interface with speech bubble-style panel design.

## Current State Assessment

### ✅ What We Have (Reusable)
- **Core Models**: Project, Taskspace, TaskspaceLog data structures
- **ProjectManager**: Complete project and taskspace management logic
- **IPC System**: Working daemon communication and MCP server integration
- **Window Registration**: CGWindowID tracking and screenshot capture
- **SwiftUI Views**: ProjectView, TaskspaceCard, all existing UI components
- **Screenshot System**: ScreenshotManager with ScreenCaptureKit integration
- **Settings & Permissions**: Complete agent selection and permission management

### 🔄 What Changes
- **Window Architecture**: Multi-window → Single splash window + floating dock panel
- **Panel Presentation**: Always-visible project windows → Dock-click activated panel
- **Project Model**: Multi-project support → Single active project at a time
- **Visual Design**: Standard windows → Speech bubble panel with dock arrow

### ➕ What We Need to Add
- **NSPanel Integration**: Custom panel with system contextual menu styling
- **Dock Interaction**: Click detection and panel positioning logic
- **Panel Lifecycle**: Show/hide behavior with proper focus management
- **Taskspace States**: Two-dimensional Active/Dormant + Hatchling/Resume state model

## Implementation Phases

### Phase 10: NSPanel Foundation 🏗️

**10.1: Create Custom NSPanel Class**
- [ ] Create `DockPanel.swift` - custom NSPanel subclass
- [ ] Configure panel styling: `.nonactivatingPanel`, floating level, no title bar
- [ ] Add visual styling to match system contextual menus (blur, rounded corners, shadow)
- [ ] Implement arrow/tail drawing pointing toward dock

**10.2: SwiftUI Integration**
- [ ] Create `DockPanelHostingView` - NSHostingView wrapper for SwiftUI content
- [ ] Port existing ProjectView to work inside NSPanel sizing constraints
- [ ] Test SwiftUI view lifecycle within NSPanel container
- [ ] Ensure proper sizing and layout for ~400px panel width

**10.3: Basic Panel Management**
- [ ] Create `DockPanelManager` - handles panel show/hide logic
- [ ] Add panel positioning logic relative to dock location
- [ ] Implement click-outside-to-dismiss behavior
- [ ] Add basic dock click detection (temporary implementation)

**Success Criteria**: NSPanel appears with SwiftUI content, positioned near dock, dismisses properly

### Phase 20: Application Architecture Changes 🔧

**20.1: Modify App.swift Window Structure**
- [ ] Remove "project" WindowGroup (multi-project window support)
- [ ] Keep "splash" WindowGroup as dedicated Project Selection Window
- [ ] Integrate DockPanelManager into app lifecycle
- [ ] Add dock click event handling to App.swift

**20.2: Single Active Project Model**
- [ ] Add `activeProjectPath` to SettingsManager (replace multi-project tracking)
- [ ] Modify SplashView to automatically restore last active project on startup
- [ ] Add "Close Project" functionality that returns to SplashView
- [ ] Update project selection flow to set active project instead of opening windows

**20.3: Splash Window as Project Manager**
- [ ] Modify SplashView to handle both project selection AND dock panel management
- [ ] Add dock panel integration to SplashView lifecycle
- [ ] Ensure splash window persists while project is active (hidden but not closed)
- [ ] Handle splash window show/hide when projects are opened/closed

**Success Criteria**: App launches with splash window, can open project that shows dock panel, can close project to return to splash

### Phase 30: Enhanced Taskspace State Management 📊

**30.1: Two-Dimensional State Model**
- [ ] Add `PersistentState` enum to Taskspace model: `Hatchling`, `Resume`
- [ ] Add runtime Active/Dormant detection via IPC roll-call system
- [ ] Modify TaskspaceCard to show visual states based on both dimensions
- [ ] Update taskspace.json persistence to include persistent state

**30.2: IPC Roll-Call System**
- [ ] Implement periodic taskspace detection broadcasts
- [ ] Add IPC message types for taskspace presence detection
- [ ] Update VSCode extension to respond to roll-call messages
- [ ] Handle taskspace Active→Dormant transitions when VSCode closes

**30.3: Visual State Indicators**
- [ ] Update TaskspaceCard to show live/grey/placeholder screenshots
- [ ] Add loading states for taskspace awakening process
- [ ] Implement visual feedback for dormant taskspaces being awakened
- [ ] Add state indicators (Active/Dormant badges) to taskspace cards

**Success Criteria**: Taskspaces correctly show Active/Dormant states, visual feedback matches actual window states

### Phase 40: Project Lifecycle & Window Management 🪟

**40.1: Graceful Project Transitions**
- [ ] Implement project close functionality that closes all VSCode windows
- [ ] Add "are you sure?" handling for unsaved work in VSCode
- [ ] Handle cleanup when switching between projects
- [ ] Preserve taskspace state across project open/close cycles

**40.2: Enhanced Window Management**
- [ ] Add TaskspaceCard click handling to focus/awaken taskspaces
- [ ] Implement window focus logic using CGWindowID references
- [ ] Handle cases where registered windows no longer exist
- [ ] Add visual feedback during window operations

**40.3: Panel UI Polish**
- [ ] Optimize TaskspaceCard layout for panel width constraints
- [ ] Add smooth animations for panel appearance/dismissal
- [ ] Implement panel positioning with screen edge awareness
- [ ] Add keyboard shortcuts for panel operations

**Success Criteria**: Complete project lifecycle, click-to-focus works, polished panel interactions

### Phase 50: Dock Integration & Polish ✨

**50.1: Proper Dock Click Detection**
- [ ] Research and implement reliable dock click detection
- [ ] Replace temporary dock click implementation with production solution
- [ ] Handle multiple dock click scenarios (app already running, not running, etc.)
- [ ] Add dock badge integration for attention requests

**50.2: Advanced Panel Behaviors**
- [ ] Implement panel auto-positioning based on dock location
- [ ] Add panel arrow that points to exact dock click location
- [ ] Handle multiple monitor scenarios
- [ ] Add panel persistence (remember size/position preferences)

**50.3: Final Polish & Testing**
- [ ] Comprehensive testing of all workflow scenarios
- [ ] Performance optimization (screenshot capture, panel animations)
- [ ] Error handling and edge case management
- [ ] User experience refinements based on testing

**Success Criteria**: Production-ready dock-activated interface with polished UX

## Technical Architecture

### Component Relationships
```
SplashView (Project Selection Window)
    ├── DockPanelManager
    │   └── DockPanel (NSPanel)
    │       └── DockPanelHostingView (NSHostingView)
    │           └── ProjectView (SwiftUI) [REUSED]
    │               └── TaskspaceCard (SwiftUI) [REUSED]
    │
    └── ProjectManager [REUSED]
        ├── IpcManager [REUSED]
        ├── ScreenshotManager [REUSED]
        └── Window Registration [REUSED]
```

### Key Classes to Create
- **DockPanel**: NSPanel subclass with contextual menu styling
- **DockPanelManager**: Panel lifecycle and positioning management
- **DockPanelHostingView**: NSHostingView wrapper for SwiftUI integration

### Key Classes to Modify
- **App.swift**: Remove project WindowGroup, add dock panel management
- **SplashView**: Become permanent project manager window
- **SettingsManager**: Add single active project tracking
- **Taskspace.swift**: Add persistent state enum and runtime state tracking

### Key Classes to Reuse Unchanged
- **ProjectManager**: Core project management logic
- **ProjectView**: Main taskspace display (fits perfectly in panel)
- **TaskspaceCard**: Individual taskspace UI (minor sizing adjustments only)
- **ScreenshotManager**: Window screenshot capture system
- **IpcManager**: MCP server communication

## Risk Mitigation

### Technical Risks
**Risk**: NSPanel + SwiftUI integration issues
**Mitigation**: Start with simple SwiftUI content, gradually add complexity

**Risk**: Dock click detection reliability
**Mitigation**: Implement temporary solution first, research proper APIs separately

**Risk**: Panel positioning on multiple monitors
**Mitigation**: Test early on multi-monitor setups, provide fallback positioning

**Risk**: SwiftUI layout issues in constrained panel width
**Mitigation**: Test TaskspaceCard layout early, adjust sizing incrementally

### User Experience Risks
**Risk**: Panel feels disconnected from dock interaction
**Mitigation**: Ensure arrow pointing and positioning are precise

**Risk**: Loss of workflow when transitioning from multi-window
**Mitigation**: Preserve all existing functionality, just change presentation

## Success Metrics

### Functional Requirements
- [ ] Single dock click shows panel with all active project taskspaces
- [ ] Panel appears positioned near dock with arrow pointing to click location
- [ ] Panel dismisses when clicking outside or pressing escape
- [ ] All existing taskspace functionality preserved (creation, logs, screenshots)
- [ ] Project switching works cleanly (close VSCode windows, return to splash)
- [ ] Taskspaces show correct Active/Dormant states with appropriate visual feedback

### Visual Requirements
- [ ] Panel matches macOS contextual menu styling (blur, rounded corners, shadow)
- [ ] Arrow/tail points accurately toward dock
- [ ] TaskspaceCard layout works well in ~400px panel width
- [ ] Smooth animations for panel show/hide
- [ ] Clear visual states for taskspace Active/Dormant/Hatchling/Resume combinations

### Performance Requirements
- [ ] Panel appears within 200ms of dock click
- [ ] Screenshot updates don't impact panel responsiveness
- [ ] Memory usage remains reasonable with panel open/closed cycles
- [ ] No memory leaks from NSPanel/SwiftUI integration

## Next Steps

1. **Start with Phase 10.1**: Create basic DockPanel NSPanel subclass
2. **Test early and often**: Verify NSPanel + SwiftUI integration works
3. **Incremental approach**: Get basic panel working before adding complex features
4. **Preserve existing code**: Reuse current SwiftUI views wherever possible

The beauty of this plan is that most of your hard work (IPC, window management, screenshot capture) stays exactly the same - we're just changing how the UI is presented to the user.

Ready to start with creating the DockPanel NSPanel class?