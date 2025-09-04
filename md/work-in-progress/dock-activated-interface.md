# Dock-Activated Interface Design

## Overview

Replace the current persistent window architecture with a dock-activated popover interface, similar to macOS Finder folder stacks. This provides a cleaner, less intrusive user experience for a developer tool.

## Current Problems

1. **SwiftUI `openWindow` is broken** - completely non-functional in our app context
2. **Window management complexity** - splash screens, multiple windows, coordination issues
3. **Screen clutter** - persistent windows take up valuable screen real estate
4. **Unfamiliar UX** - most developer tools don't use persistent control windows

## Proposed Solution

### User Experience
- **No persistent windows** - app runs silently in background
- **Dock click activation** - clicking dock icon shows interface popover
- **Contextual positioning** - popover appears near dock, similar to folder stacks
- **Auto-dismiss** - interface disappears when task complete or clicking outside
- **Quick access** - always available but never in the way

### Technical Architecture

#### App Lifecycle
```swift
// App runs as accessory (no dock icon when no windows)
NSApp.setActivationPolicy(.accessory)

// Handle dock clicks to show interface
func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
    showInterface()
    return false
}
```

#### Interface Panel
- **NSPanel** with `.nonactivatingPanel` style
- **Floating behavior** - stays on top but doesn't steal focus
- **Smart positioning** - appears near dock, adjusts for screen edges
- **Compact design** - optimized for quick interactions

#### Content Architecture
```
DockInterface
├── ProjectSelection (when no project loaded)
├── ProjectOverview (when project active)
│   ├── Taskspace list
│   ├── Quick actions
│   └── Status indicators
└── Settings (accessible from any state)
```

## Implementation Plan

### Phase 1: Basic Dock Activation
1. Remove all WindowGroups except settings sheet
2. Implement `NSPanel`-based interface
3. Add dock click handler
4. Basic positioning logic

### Phase 2: Interface Content
1. Port existing ProjectSelectionView to panel
2. Create compact ProjectOverview
3. Implement auto-dismiss behavior
4. Add smooth show/hide animations

### Phase 3: Polish
1. Smart positioning (multi-monitor, dock position)
2. Keyboard shortcuts for quick access
3. Remember panel size/position preferences
4. Accessibility support

## Benefits

### User Experience
- **Familiar pattern** - matches macOS folder stack behavior
- **Non-intrusive** - no persistent windows
- **Quick access** - always one click away
- **Context-aware** - shows relevant content based on state

### Technical
- **Eliminates `openWindow` dependency** - uses working NSPanel APIs
- **Simpler architecture** - single interface, no window coordination
- **Better resource usage** - interface only exists when needed
- **Easier testing** - single UI component to test

### Development
- **Faster iteration** - no complex window state management
- **Cleaner code** - single interface component
- **Better debugging** - all UI logic in one place

## Design Considerations

### Panel Sizing
- **Compact by default** - ~400x600px
- **Resizable** - user can adjust for their workflow
- **Content-adaptive** - grows/shrinks based on content

### Positioning Logic
```
1. Detect dock position (bottom/left/right)
2. Calculate available screen space
3. Position panel near dock with padding
4. Adjust for screen edges and menu bar
5. Remember user's preferred position
```

### Auto-Dismiss Behavior
- **Click outside** - hide panel
- **Task completion** - hide after project opened
- **Escape key** - hide panel
- **App deactivation** - hide panel

### Content States
1. **No project** - Show project selection
2. **Project loading** - Show progress indicator
3. **Project active** - Show taskspace overview
4. **Error state** - Show error with retry options

## Migration Strategy

### Backward Compatibility
- Keep existing project data structures
- Reuse existing business logic (ProjectManager, etc.)
- Maintain same keyboard shortcuts where applicable

### Gradual Rollout
1. **Feature flag** - allow switching between old/new interface
2. **User preference** - let users choose interface style
3. **Feedback period** - gather user input before full migration
4. **Remove old code** - clean up after successful migration

## Technical Details

### NSPanel Configuration
```swift
panel.styleMask = [.nonactivatingPanel, .titled, .closable, .resizable]
panel.level = .floating
panel.hidesOnDeactivate = true
panel.animationBehavior = .utilityWindow
```

### SwiftUI Integration
```swift
// Host SwiftUI content in NSPanel
let hostingView = NSHostingView(rootView: DockInterfaceView())
panel.contentView = hostingView
```

### Positioning Algorithm
```swift
func calculatePanelPosition() -> NSPoint {
    let dockPosition = getDockPosition()
    let screenFrame = NSScreen.main?.visibleFrame ?? .zero
    
    switch dockPosition {
    case .bottom:
        return NSPoint(x: screenFrame.midX - panelWidth/2, 
                      y: screenFrame.minY + dockHeight + padding)
    case .left:
        return NSPoint(x: screenFrame.minX + dockWidth + padding,
                      y: screenFrame.midY - panelHeight/2)
    // ... etc
    }
}
```

## Success Metrics

### User Experience
- **Reduced clicks** to access functionality
- **Faster task completion** times
- **Less screen clutter** reported by users
- **Higher user satisfaction** scores

### Technical
- **Zero `openWindow` dependencies**
- **Reduced crash reports** from window management
- **Faster app startup** (no window initialization)
- **Lower memory usage** (interface created on demand)

## Risks and Mitigations

### Risk: Unfamiliar UX Pattern
- **Mitigation**: Provide onboarding tooltip on first use
- **Mitigation**: Include traditional menu bar access as fallback

### Risk: Positioning Issues on Multi-Monitor
- **Mitigation**: Extensive testing on various monitor configurations
- **Mitigation**: Fallback to center-screen positioning if detection fails

### Risk: Accessibility Concerns
- **Mitigation**: Full VoiceOver support for panel navigation
- **Mitigation**: Keyboard-only operation support

## Future Enhancements

### Menu Bar Integration
- Optional menu bar icon for quick access
- Status indicators in menu bar (active taskspaces, etc.)

### Quick Actions
- Keyboard shortcuts to show panel and perform common actions
- Spotlight-style search within panel

### Contextual Content
- Show different content based on current active application
- Integration with IDE state (current file, git status, etc.)

## Conclusion

The dock-activated interface solves multiple current problems while providing a more native macOS experience. It eliminates the broken `openWindow` dependency, reduces screen clutter, and follows familiar interaction patterns that users already understand.

This approach transforms Symposium from a traditional multi-window application into a modern, unobtrusive developer tool that's always available but never in the way.
