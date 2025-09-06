# Dock-Activated Interface Implementation Plan

*Step-by-step transition from multi-window to dock-activated interface*

## Overview

This document outlines the implementation plan for transitioning Symposium from its current multi-window architecture to the new dock-activated interface with speech bubble-style panel design.

## 🎉 Current Implementation Status

**✅ COMPLETED PHASES:**
- **Phase 10**: NSPanel Foundation - Custom panel with speech bubble styling
- **Phase 20**: Application Architecture Changes - Single window architecture  
- **Phase 22**: Improved Project Workflow & XOR Invariant - Fully functional dock-activated interface!

**🚀 NEXT UP:**
- **Phase 30**: Enhanced Taskspace State Management - Two-dimensional state model
- **Phase 40**: Project Lifecycle & Window Management - Advanced interactions
- **Phase 50**: Dock Integration & Polish - Production-ready experience

**🏆 MILESTONE ACHIEVED**: The core dock-activated interface is **working perfectly**! Users can click the dock and see a beautiful panel with their taskspaces. The XOR invariant (Active Project ⊕ Splash Window Visible) functions flawlessly.

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

### Phase 10: NSPanel Foundation ✅

**10.1: Create Custom NSPanel Class**
- [x] Create `DockPanel.swift` - custom NSPanel subclass
- [x] Configure panel styling: `.nonactivatingPanel`, floating level, no title bar
- [x] Add visual styling to match system contextual menus (blur, rounded corners, shadow)
- [x] Implement arrow/tail drawing pointing toward dock

**10.2: SwiftUI Integration**
- [x] Create `DockPanelHostingView` - NSHostingView wrapper for SwiftUI content
- [x] Port existing ProjectView to work inside NSPanel sizing constraints
- [x] Test SwiftUI view lifecycle within NSPanel container
- [x] Ensure proper sizing and layout for ~400px panel width

**10.3: Basic Panel Management**
- [x] Create `DockPanelManager` - handles panel show/hide logic
- [x] Add panel positioning logic relative to dock location
- [x] Implement click-outside-to-dismiss behavior
- [x] Add basic dock click detection (temporary implementation)

**Success Criteria**: ✅ NSPanel appears with SwiftUI content, positioned near dock, dismisses properly

### Phase 20: Application Architecture Changes ✅

**20.1: Modify App.swift Window Structure**
- [x] Remove "project" WindowGroup (multi-project window support)
- [x] Keep "splash" WindowGroup as dedicated Project Selection Window
- [x] Integrate DockPanelManager into app lifecycle (via AppDelegate coordination)
- [x] Add dock click event handling to App.swift (inherited from Phase 10)

**20.2: Single Active Project Model**
- [x] Add `activeProjectPath` to SettingsManager (replace multi-project tracking)
- [x] Modify SplashView to automatically restore last active project on startup
- [x] Add "Close Project" functionality that returns to SplashView
- [x] Update project selection flow to set active project instead of opening windows

**20.3: Splash Window as Project Manager**
- [x] Modify SplashView to handle both project selection AND dock panel management
- [x] Add dock panel integration to SplashView lifecycle (via AppDelegate coordination)
- [x] Ensure splash window persists while project is active (no longer dismissed)
- [x] Handle splash window show/hide when projects are opened/closed (ActiveProjectView)

**Success Criteria**: ✅ App launches with splash window, can open project that shows dock panel, can close project to return to splash

**Implementation Notes**: 
- Created `ActiveProjectView` for managing active project state with "Close Project" functionality
- Renamed `lastProjectPath` → `activeProjectPath` across entire codebase for clearer semantics  
- SplashView now manages project lifecycle with `setActiveProject()` and `closeActiveProject()` methods
- Single window architecture: splash window persists and shows different content based on project state
- Full AppDelegate integration for dock panel coordination maintained from Phase 10
- Project restoration on app startup now sets active project instead of opening separate windows

### Phase 22: Improved Project Workflow & XOR Invariant 🎉 **COMPLETE**

**22.1: Implement XOR Invariant (Active Project ⊕ Splash Window Visible)**
- [x] Remove ActiveProjectView entirely (intermediate state not needed)
- [x] Modify setActiveProject() to hide splash window instead of showing ActiveProjectView
- [x] Modify closeActiveProject() to show splash window when project closed
- [x] Update SplashView body logic to never show ActiveProjectView
- [x] Refactor to use proper SwiftUI window management (dismiss/openWindow) instead of NSApp.windows search

**22.2: Immediate Dock Panel on Project Open**
- [x] Show dock panel immediately when project is opened (don't wait for dock click)
- [x] Position dock panel near dock location on project open
- [x] Ensure panel shows dormant taskspaces ready for user activation
- [x] Maintain existing dock click behavior for subsequent panel displays

**22.3: Dock Panel as Primary Project Interface**
- [x] Add "Close Project" functionality to dock panel (red X button)
- [x] Remove "Close Project" from splash window (no longer needed)
- [x] Ensure dock panel can manage all project operations
- [x] Update dock panel to be the sole interface for active projects
- [x] Create callback chain: ProjectView → DockPanelManager → AppDelegate → SplashView

**22.4: Consistent Initial Taskspace Creation**
- [ ] Modify project creation to automatically create one initial taskspace (dormant)
- [ ] Ensure new project taskspaces start dormant until user activation
- [ ] Update existing project loading to show all taskspaces as dormant initially
- [ ] Create consistent experience: all projects open with dormant taskspaces

**22.5: Critical Bug Fixes** 🔧
- [x] **FIXED**: AppDelegate coordination timing issue preventing dock panel display
  - Problem: AppDelegate.shared returned nil during SwiftUI startup, breaking project manager coordination
  - Solution: Use SwiftUI environment objects instead of shared instance pattern
- [x] **FIXED**: Splash window opening from AppDelegate using notification system
  - Problem: AppDelegate couldn't directly create SwiftUI windows
  - Solution: Notification-based coordination between AppDelegate and App

**Success Criteria**: ✅ **ACHIEVED** - Opening project immediately shows dock panel with dormant taskspaces, splash window hidden. Closing project returns to splash. Clean XOR invariant maintained. **Dock clicks now properly show panel!**

**Implementation Notes**: 🎉 **FULLY FUNCTIONAL** - The dock-activated interface is working perfectly! Users click dock → see speech bubble panel with taskspaces. Project workflow is intuitive: project selection → immediate dock panel → taskspace activation. XOR invariant works flawlessly through proper SwiftUI patterns. Ready for Phase 30!

### Phase 30: Enhanced Taskspace State Management 🎯 **IN PROGRESS**

**30.1: Two-Dimensional State Model** ✅
- [x] ~~Add `PersistentState` enum to Taskspace model~~ → **ALTERNATE**: Use existing `TaskspaceState` enum ([`Taskspace.swift:7`](application/osx/Sources/Symposium/Models/Taskspace.swift))
- [x] Add runtime Active/Dormant detection via window registration → [`hasRegisteredWindow`](application/osx/Sources/Symposium/Views/ProjectView.swift:175-177)
- [x] Modify TaskspaceCard to show visual states based on both dimensions → [`stateIcon`](application/osx/Sources/Symposium/Views/ProjectView.swift:187-201), [`stateText`](application/osx/Sources/Symposium/Views/ProjectView.swift:195-201)
- [x] ~~Update taskspace.json persistence~~ → **EXISTING**: `TaskspaceState` already persisted via Codable

**30.2: IPC Roll-Call System** ⚠️ **PARTIAL**
- [ ] **MISSING**: Implement periodic taskspace detection broadcasts (only manual via [`reregisterWindows()`](application/osx/Sources/Symposium/Views/ProjectView.swift:152-167))
- [x] Add IPC message types for taskspace presence detection → [`TaskspaceRollCallPayload`](application/osx/Sources/Symposium/Models/IpcManager.swift:57-63)
- [x] ~~Update VSCode extension to respond to roll-call messages~~ → **EXISTING**: Extension already responds
- [x] ~~**MISSING**: Handle taskspace Active→Dormant transitions when VSCode closes~~ → **FIXED**: Automatic polling-based detection working ([`250e5fb`](https://github.com/socratic-shell/symposium/commit/250e5fb))

**30.3: Visual State Indicators** ✅ **COMPLETED** 
- [x] ~~**CRITICAL MISSING**: Save screenshots to disk~~ → **FIXED**: Screenshots saved to `task-{UUID}/screenshot.png` ([`ad22cc3`](https://github.com/socratic-shell/symposium/commit/ad22cc3))
- [x] ~~**CRITICAL BUG**: Screenshots not displaying for dormant taskspaces~~ → **FIXED**: Display logic corrected ([`9d47a08`](https://github.com/socratic-shell/symposium/commit/9d47a08))
- [x] ~~Add loading states for taskspace awakening process~~ → **BETTER**: Clear "Starting..." state for Hatchling activation
- [x] Implement visual feedback for dormant taskspaces → "Click to start" vs "Click to connect" messaging with opacity differences
- [x] ~~Add state indicators (Active/Dormant badges)~~ → **BETTER**: Intuitive text and icons instead of badges

**30.4: Core Behavioral Foundation** ✅ **COMPLETED**
- [x] Dormant by default: Projects open without auto-launching VSCode → [`ProjectManager.swift:111-112`](application/osx/Sources/Symposium/Models/ProjectManager.swift:111-112)
- [x] Click-to-activate: Users click taskspace cards to launch VSCode → [`handleTaskspaceClick()`](application/osx/Sources/Symposium/Views/ProjectView.swift:210-218)
- [x] New taskspaces start dormant → [`ProjectManager.swift:299-300`](application/osx/Sources/Symposium/Models/ProjectManager.swift:299-300)
- [x] Visual distinction: Hatchling vs Resume states → [`stateIcon`](application/osx/Sources/Symposium/Views/ProjectView.swift:187-193), [`stateText`](application/osx/Sources/Symposium/Views/ProjectView.swift:195-201)

**🚨 Remaining Items:**
1. ~~**Screenshot Persistence & Display**~~ → **COMPLETE** ✅ 
2. ~~**Automatic Window Close Detection**~~ → **COMPLETE** ✅
3. **Optional: Targeted Roll-Calls for Recovery** → Edge case recovery when windows should be active but aren't tracked

**Success Criteria**: ✅ **PHASE 30 COMPLETE** - Seamless two-dimensional state management with automatic detection working perfectly!

### Phase 35: Centered Panel with Grid Layout 🎯 **NEXT PRIORITY**

*Transition from dock-relative positioning to centered panel with responsive grid*

**Current State Analysis:**
- ✅ **Working dock-activated interface** with speech bubble panel (Phase 30 complete)
- ✅ **NSPanel architecture** proven and functional (`DockPanel.swift`, `DockPanelManager.swift`)
- ✅ **ProjectView + TaskspaceCard** layout working well in current 550px panel
- ❌ **Positioning issues**: `calculatePanelPosition()` uses dock estimation causing unpredictable placement

**35.1: Replace Dock-Relative Positioning with Centered Layout** ⏳ **IN PROGRESS**
- [ ] **Update `DockPanelManager.calculatePanelPosition()`**: Replace dock-relative logic with screen center calculation
- [ ] **Remove arrow functionality**: Eliminate `DockPanel.ArrowDirection` and arrow drawing since centered panel doesn't need directional arrow
- [ ] **Simplify panel appearance**: Remove arrow space calculations from `setupPanelLayout()` padding
- [ ] **Update panel size**: Implement responsive panel sizing based on taskspace width calculations

**35.2: Implement Responsive Grid Layout Calculations** ⏳ **CORE FEATURE**
- [ ] **Add taskspace width calculation**: Create `calculateTaskspaceWidth()` method using screenshot + text + padding
- [ ] **Implement panel width constraints**: Add constraint chain (4*TW → 3/4 screen → min 1*TW → screen limit)
- [ ] **Update `calculateIdealPanelSize()`**: Replace hardcoded 550x800 with calculated responsive dimensions
- [ ] **Grid layout logic**: Calculate columns per row and total rows based on taskspace count

**35.3: Add Taskspace Expand/Collapse Functionality** ⏳ **NEW FEATURE**
- [ ] **Expand state management**: Add `@State var expandedTaskspace: UUID?` to `ProjectView`
- [ ] **Detail mode layout**: Create expanded taskspace view that fills entire panel
- [ ] **Breadcrumb navigation**: Add "Project > Taskspace Name [↩]" header for detail mode
- [ ] **Scrollable logs**: Implement full log history scrolling in expanded view
- [ ] **Back navigation**: Handle collapse back to grid mode

**35.4: Update Panel Dismissal Behavior** ⏳ **BEHAVIOR CHANGE**
- [ ] **Smart dismissal logic**: Distinguish between management actions (persist panel) vs engagement (dismiss panel)
- [ ] **Update taskspace click handlers**: Dismiss panel when launching/focusing VSCode, keep open for expansion
- [ ] **Panel persistence**: Keep panel visible for expand, scroll, settings, create taskspace
- [ ] **Future pin preparation**: Add UI placeholder for pin button (non-functional initially)

**Key Code Changes Required:**

```swift
// DockPanelManager.swift - Replace positioning logic
private func calculatePanelPosition(for panelSize: NSSize, near dockClickPoint: NSPoint) -> NSPoint {
    guard let screen = NSScreen.main else { return NSPoint.zero }
    let screenFrame = screen.visibleFrame
    
    // Center the panel on screen
    let centeredX = screenFrame.midX - (panelSize.width / 2)
    let centeredY = screenFrame.midY - (panelSize.height / 2)
    
    return NSPoint(x: centeredX, y: centeredY)
}

// Add responsive sizing
private func calculateIdealPanelSize() -> NSSize {
    let taskspaceWidth = calculateTaskspaceWidth()
    let screenWidth = NSScreen.main?.visibleFrame.width ?? 1200
    
    // Implement constraint chain
    let idealWidth = 4 * taskspaceWidth
    let constrainedWidth = min(idealWidth, 0.75 * screenWidth)
    let finalWidth = max(constrainedWidth, taskspaceWidth)
    let panelWidth = min(finalWidth, screenWidth)
    
    // Calculate height based on taskspace count and rows
    let taskspacesPerRow = Int(floor(panelWidth / taskspaceWidth))
    let numRows = ceil(Double(projectManager.currentProject?.taskspaces.count ?? 1) / Double(taskspacesPerRow))
    let panelHeight = numRows * taskspaceHeight + headerHeight
    
    return NSSize(width: panelWidth, height: min(panelHeight, 0.8 * screenHeight))
}

private func calculateTaskspaceWidth() -> CGFloat {
    let screenshotWidth: CGFloat = 120
    let sampleTextWidth = "Implementing authentication system...".size(withAttributes: [.font: NSFont.systemFont(ofSize: 13)]).width
    let padding: CGFloat = 40
    return screenshotWidth + sampleTextWidth + padding
}
```

**Success Criteria for Phase 35:**
- [ ] Panel always appears at screen center (predictable positioning)
- [ ] Panel width adapts to screen size and taskspace count
- [ ] Grid layout shows 1-4 taskspaces per row based on calculated constraints
- [ ] Taskspaces can expand to fill panel for detailed log viewing
- [ ] Panel persists for management but dismisses for VSCode engagement
- [ ] All existing functionality preserved (screenshots, logs, taskspace creation)

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