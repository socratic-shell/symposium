# Centered Panel Implementation Plan

*Commit-sized steps to transition from dock-relative positioning to centered panel with responsive grid*

## Current State Analysis

### Working Components (Phase 30 Complete)
- ✅ **Dock-activated interface** functional with speech bubble panel
- ✅ **Two-dimensional taskspace state** (Active/Dormant + Hatchling/Resume)
- ✅ **Screenshot capture and persistence** working
- ✅ **IPC communication** with VSCode extension established

### Current Architecture
```
DockPanelManager.swift:169     - calculateIdealPanelSize(): hardcoded NSSize(width: 550, height: 800)
DockPanelManager.swift:172     - calculatePanelPosition(): dock-relative positioning logic
ProjectView.swift              - VStack layout with taskspace cards
TaskspaceCard.swift:187-201    - stateIcon/stateText logic for visual states
DockPanel.swift                - NSPanel with arrow drawing functionality
```

### Core Problems to Fix
- **Unpredictable positioning**: `calculatePanelPosition()` uses dock estimation heuristics
- **Fixed sizing**: Hardcoded 550x800 doesn't adapt to content or screen size
- **No grid layout**: Current VStack doesn't support responsive columns
- **No expand functionality**: Can't view detailed logs within panel

## Implementation Steps

### Step 1: Replace Dock-Relative Positioning
**Goal**: Panel always appears at screen center  
**Files**: `DockPanelManager.swift`

```swift
// Replace calculatePanelPosition() method
private func calculatePanelPosition(for panelSize: NSSize, near dockClickPoint: NSPoint) -> NSPoint {
    guard let screen = NSScreen.main else { return NSPoint.zero }
    let screenFrame = screen.visibleFrame
    
    // Center the panel on screen
    let centeredX = screenFrame.midX - (panelSize.width / 2)
    let centeredY = screenFrame.midY - (panelSize.height / 2)
    
    return NSPoint(x: centeredX, y: centeredY)
}
```

**Testing Scenarios**:
- **App startup**: Launch app, it restores previous project, click dock → panel appears at screen center
- **Multiple monitors**: Test on different monitor configurations → panel always centers on main display
- **Dock position**: Try with dock on bottom, left, right → panel position unchanged (always centered)
- **Screen resize**: Resize screen while panel open → panel stays centered

---

### Step 2: Add Taskspace Width Calculation
**Goal**: Calculate responsive panel width based on content  
**Files**: `DockPanelManager.swift`

```swift
private func calculateTaskspaceWidth() -> CGFloat {
    let screenshotWidth: CGFloat = 120
    
    // Measure sample Star Trek log message
    let sampleText = "Captain, we're getting mysterious sensor readings"
    let textAttributes = [NSAttributedString.Key.font: NSFont.systemFont(ofSize: 13)]
    let sampleTextWidth = sampleText.size(withAttributes: textAttributes).width
    
    let padding: CGFloat = 40 // Internal card padding
    return screenshotWidth + sampleTextWidth + padding
}
```

**Testing Scenarios**:
- **App startup**: Launch app, restore project, open panel → taskspace cards have consistent, reasonable width
- **Log message verification**: Check that calculated width can fit "Captain, we're getting mysterious sensor readings" plus screenshot
- **Console logging**: Add temporary logs to verify calculated width values are sensible (e.g., ~300-400px)
- **Visual inspection**: Cards should not be too narrow or excessively wide compared to current 550px panel

---

### Step 3: Implement Panel Width Constraint System
**Goal**: Responsive panel sizing based on screen and content  
**Files**: `DockPanelManager.swift`

```swift
private func calculateIdealPanelSize() -> NSSize {
    guard let screen = NSScreen.main else { return NSSize(width: 550, height: 800) }
    
    let taskspaceWidth = calculateTaskspaceWidth()
    let screenFrame = screen.visibleFrame
    
    // Panel Width Constraint Chain
    let idealWidth = 4 * taskspaceWidth              // Target 4 taskspaces per row
    let screenConstraint = 0.75 * screenFrame.width  // Max 3/4 screen width  
    let constrainedWidth = min(idealWidth, screenConstraint)
    let finalWidth = max(constrainedWidth, taskspaceWidth) // Min 1 taskspace width
    let panelWidth = min(finalWidth, screenFrame.width)    // Hard screen limit
    
    // Calculate height (placeholder logic)
    let panelHeight = min(800, 0.8 * screenFrame.height)
    
    return NSSize(width: panelWidth, height: panelHeight)
}
```

**Testing Scenarios**:
- **Large monitor (2560px)**: Launch app, open panel → should show 4 taskspaces per row, panel width ~75% of screen
- **Laptop screen (1440px)**: Same project → should show fewer taskspaces per row, panel width constrained to 3/4 screen
- **Small screen (1024px)**: Same project → should show minimum 1 taskspace width, potentially single column
- **Window logging**: Console should show constraint chain calculations: ideal→screen constraint→minimum→final
- **Edge case**: Very wide screen should cap at 4 taskspaces per row, not expand infinitely

---

### Step 4: Remove Arrow Functionality  
**Goal**: Simplify panel appearance for centered positioning  
**Files**: `DockPanel.swift`, `DockPanelManager.swift`

**In DockPanelManager.swift**:
```swift
// Remove arrow-related parameters from showPanel()
panel.showPanel(at: panelPosition) // Remove arrow direction/position
```

**In DockPanel.swift**:
- Remove `ArrowDirection` enum and arrow drawing code
- Remove `setArrowDirection()` method
- Simplify `setupVisualEffectView()` to remove arrow space calculations

**Testing Scenarios**:
- **App startup**: Launch app, restore project, click dock → panel appears as clean rounded rectangle (no arrow pointing anywhere)
- **Visual comparison**: Panel should look like a standard macOS contextual menu or popover, not a speech bubble
- **Multiple dock positions**: Test with dock on different sides → no arrow artifacts or pointing elements
- **Panel edges**: All edges should be consistently rounded, no asymmetrical shapes
- **Shadow/blur**: Panel should maintain system-standard drop shadow and blur effects

---

### Step 5: Add Expand/Collapse State Management
**Goal**: Support detail mode for individual taskspaces  
**Files**: `ProjectView.swift`

```swift
struct ProjectView: View {
    @ObservedObject var projectManager: ProjectManager
    @ObservedObject var ipcManager: IpcManager
    var onCloseProject: (() -> Void)?
    
    @State private var expandedTaskspace: UUID? = nil // NEW
    
    var body: some View {
        VStack {
            // Header (always visible)
            headerView
            
            if let expandedTaskspace = expandedTaskspace {
                // Detail mode - show expanded taskspace
                expandedTaskspaceView(for: expandedTaskspace)
            } else {
                // Grid mode - show all taskspaces
                taskspaceGridView
            }
            
            // Footer (always visible)  
            footerView
        }
    }
    
    private var headerView: some View {
        // Existing header logic with breadcrumb when expanded
    }
    
    private func expandedTaskspaceView(for taskspaceId: UUID) -> some View {
        // New expanded view with scrollable logs
    }
    
    private var taskspaceGridView: some View {
        // Existing taskspace grid (to be enhanced in next step)
    }
    
    private var footerView: some View {
        HStack {
            Button(action: { /* create taskspace */ }) {
                HStack {
                    Image(systemName: "plus")
                    Text("New Taskspace")
                }
            }
            Spacer()
        }
        .padding()
        .background(Color.gray.opacity(0.1))
    }
}
```

**Testing Scenarios**:
- **App startup**: Launch app, restore project, click dock → shows grid mode with all taskspaces visible
- **Expand taskspace**: Click expand button on any taskspace → panel switches to detail mode showing breadcrumb "Project: MyApp > Taskspace Name [↩]"
- **View logs**: In detail mode, should see scrollable list of that taskspace's log entries
- **Back navigation**: Click [↩] button → returns to grid mode showing all taskspaces
- **State persistence**: Expand taskspace, click outside to dismiss panel, reopen panel → should return to grid mode (not remember expansion)
- **Multiple taskspaces**: Try expanding different taskspaces → each shows its own logs and details

---

### Step 6: Implement Responsive Grid Layout
**Goal**: Dynamic columns based on calculated panel width  
**Files**: `ProjectView.swift`

```swift
private var taskspaceGridView: some View {
    let taskspaceWidth = calculateTaskspaceWidth() // Move to shared location
    let columns = calculateGridColumns(panelWidth: /* from parent */, taskspaceWidth: taskspaceWidth)
    
    LazyVGrid(columns: Array(repeating: GridItem(.fixed(taskspaceWidth)), count: columns), spacing: 16) {
        ForEach(project.taskspaces, id: \.id) { taskspace in
            TaskspaceCard(taskspace: taskspace)
                .onTapGesture {
                    handleTaskspaceClick(taskspace)
                }
        }
    }
    .padding()
}

private func calculateGridColumns(panelWidth: CGFloat, taskspaceWidth: CGFloat) -> Int {
    let availableWidth = panelWidth - 32 // Account for padding
    let maxColumns = Int(floor(availableWidth / taskspaceWidth))
    return max(1, maxColumns) // Always at least 1 column
}
```

**Testing Scenarios**:
- **App startup with multiple taskspaces**: Launch app, restore project with 6+ taskspaces, click dock → taskspaces arrange in grid (e.g., 2 rows of 3, or 3 rows of 2)
- **Screen size adaptation**: Test same project on different screen sizes → column count changes but taskspaces maintain consistent width
- **Add taskspace**: In grid mode, click "New Taskspace" → new taskspace appears in grid, layout reflows if needed
- **Single taskspace**: Project with 1 taskspace → shows single column, centered
- **Many taskspaces**: Project with 12+ taskspaces → shows multiple rows, scrollable
- **Visual consistency**: All taskspace cards should have identical width, consistent spacing between them

---

### Step 7: Update Smart Dismissal Behavior
**Goal**: Panel persists for management, dismisses for VSCode engagement  
**Files**: `ProjectView.swift`, `DockPanelManager.swift`

**In ProjectView.swift**:
```swift
private func handleTaskspaceClick(_ taskspace: Taskspace) {
    if taskspace.hasRegisteredWindow {
        // Active taskspace - focus VSCode and dismiss panel
        projectManager.focusTaskspaceWindow(taskspace)
        dismissPanel() // Signal to parent to close panel
    } else {
        // Dormant taskspace - launch VSCode and dismiss panel
        projectManager.activateTaskspace(taskspace)
        dismissPanel() // Signal to parent to close panel
    }
}

private func handleTaskspaceExpand(_ taskspace: Taskspace) {
    // Expand for management - keep panel visible
    expandedTaskspace = taskspace.id
    // No dismissPanel() call
}

private func dismissPanel() {
    // Signal to DockPanelManager to hide panel
    onCloseProject?() // Reuse existing callback or add new one
}
```

**Testing Scenarios**:
- **Launch dormant taskspace**: Click taskspace card (not expand button) → VSCode launches, panel disappears
- **Focus active taskspace**: Click active taskspace card → VSCode window comes to front, panel disappears  
- **Expand for management**: Click expand button on taskspace → panel stays open, switches to detail mode
- **Scroll logs**: In detail mode, scroll through logs → panel remains open
- **Create taskspace**: Click "New Taskspace" button → new taskspace created, panel stays open showing updated grid
- **Click outside**: Click outside panel → panel dismisses (existing behavior preserved)
- **Escape key**: Press Escape → panel dismisses (existing behavior preserved)
- **Management vs engagement**: Only VSCode launching/focusing should dismiss panel, all other interactions should keep it open

---

### Step 8: Add Panel Height Calculation
**Goal**: Responsive height based on taskspace count and screen size  
**Files**: `DockPanelManager.swift`

```swift
private func calculateIdealPanelSize() -> NSSize {
    // ... existing width calculation ...
    
    let taskspaceCount = projectManager.currentProject?.taskspaces.count ?? 1
    let taskspacesPerRow = Int(floor(panelWidth / taskspaceWidth))
    let numRows = Int(ceil(Double(taskspaceCount) / Double(taskspacesPerRow)))
    
    let taskspaceHeight: CGFloat = 120  // Approximate card height
    let headerHeight: CGFloat = 80
    let footerHeight: CGFloat = 60
    let rowSpacing: CGFloat = 16
    
    let contentHeight = CGFloat(numRows) * taskspaceHeight + CGFloat(numRows - 1) * rowSpacing
    let totalHeight = headerHeight + contentHeight + footerHeight
    
    let maxHeight = 0.8 * screenFrame.height
    let finalHeight = min(totalHeight, maxHeight)
    
    return NSSize(width: panelWidth, height: finalHeight)
}
```

**Testing Scenarios**:
- **Small project (2-4 taskspaces)**: Launch app, restore small project → panel height just fits content, no wasted space
- **Medium project (8-12 taskspaces)**: Same process → panel grows taller to accommodate all taskspaces without scrolling
- **Large project (20+ taskspaces)**: Same process → panel caps at ~80% screen height, shows scroll indication
- **Screen constraints**: Test on small laptop screen → panel never exceeds screen bounds
- **Dynamic resize**: Create new taskspaces → panel height grows incrementally
- **Height consistency**: Panel should not jump dramatically in size, smooth incremental changes

---

### Step 9: Add Scrolling Support
**Goal**: Handle overflow when many taskspaces exceed panel height  
**Files**: `ProjectView.swift`

```swift
private var taskspaceGridView: some View {
    ScrollView {
        LazyVGrid(columns: gridColumns, spacing: 16) {
            ForEach(project.taskspaces, id: \.id) { taskspace in
                TaskspaceCard(taskspace: taskspace)
                    .onTapGesture { handleTaskspaceClick(taskspace) }
            }
        }
        .padding()
    }
}
```

**Testing Scenarios**:
- **Large project (20+ taskspaces)**: Launch app, restore large project → panel shows first ~2-3 rows, scroll indicator visible at bottom
- **Scroll interaction**: Use mouse wheel/trackpad to scroll → can see all taskspaces by scrolling down
- **Scroll bounds**: Scroll to top → cannot over-scroll beyond first row; scroll to bottom → cannot over-scroll beyond last row
- **Footer visibility**: Footer with "New Taskspace" button should always be visible regardless of scrolling
- **Smooth scrolling**: Scrolling should be smooth, not jumpy or laggy with many taskspaces
- **Expand while scrolled**: Scroll down, expand a taskspace → detail mode shows, scrolling context preserved when returning to grid

---

### Step 10: Polish and Testing
**Goal**: Final integration and edge case handling  
**Files**: All modified files

**Comprehensive Testing Scenarios**:
- **Complete workflow**: Launch app → restores project → click dock → panel appears centered with responsive grid → expand taskspace → view logs → navigate back → click taskspace to launch VSCode → panel dismisses → VSCode appears focused
- **Screen variety**: Test complete workflow on MacBook (13", 1440px), external monitor (24", 2560px), and ultrawide (3440px) → behavior consistent, layout adapts appropriately
- **Project sizes**: Test with minimal project (1 taskspace), typical project (6 taskspaces), and large project (25+ taskspaces) → all render correctly with appropriate scrolling
- **Edge cases**: 
  - Very long taskspace names → cards don't break layout
  - Taskspaces with no logs → expand mode handles gracefully  
  - Mixed active/dormant taskspaces → visual states clearly differentiated
  - Rapid clicking → no UI glitches or race conditions
- **Performance**: Large project (50+ taskspaces) → panel opens quickly (<500ms), scrolling remains smooth, memory usage reasonable
- **Integration**: All existing functionality works → screenshots capture correctly, logs update in real-time, window registration functions, IPC communication intact

## Success Criteria

**Functional Requirements**:
- [ ] Panel appears at screen center consistently
- [ ] Panel width adapts to screen size (1-4 taskspaces per row)
- [ ] Taskspaces can expand for detailed log viewing
- [ ] Panel dismisses on VSCode engagement, persists for management
- [ ] Scrolling works with many taskspaces
- [ ] All existing functionality preserved (screenshots, logs, creation)

**Technical Requirements**:
- [ ] No dock-relative positioning code remains
- [ ] Responsive sizing calculations work correctly
- [ ] Expand/collapse state management is clean
- [ ] Performance acceptable with 20+ taskspaces

## Rollback Plan

Each step is designed to be independently revertible:
- Keep original methods as `_legacy` versions initially
- Test each step thoroughly before proceeding
- Maintain feature flags where possible
- Document what each commit changes for easy reversal