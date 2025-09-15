# Stacked Windows

## Overview

Stacked windows is a per-project feature that creates a "deck of cards" effect where all taskspace windows occupy the same screen position. When enabled, clicking a taskspace brings its window to the front while positioning all other taskspace windows at the exact same location behind it. This creates a clean, organized workspace where users can quickly switch between taskspaces without window clutter.

## User Experience

### Basic Behavior
- **Per-project setting**: Each project can independently enable/disable stacked windows via a checkbox in the project header
- **Persistent storage**: Setting is stored in `project.json` and travels with the project
- **Normal mode**: When disabled, clicking taskspaces focuses windows normally
- **Stacked mode**: When enabled, clicking a taskspace brings it to front and positions all other taskspace windows at the same location

### Window Following
When stacked windows is enabled and the user interacts with any window in the stack:
- **Drag following**: All stacked windows move together as a cohesive unit during drag operations
- **Resize following**: All stacked windows resize to match when any window is resized
- Following happens during the operation at 20fps (50ms intervals)
- No manual repositioning needed - the illusion of a single window is maintained

## Technical Architecture

### Core Components

#### ProjectManager Integration
- `stackedWindowsEnabled` property added to Project model (version 2)
- `setStackedWindowsEnabled()` method updates setting and saves to disk
- `focusWindowWithStacking()` implements the core stacking logic

#### WindowStackTracker
- Handles drag and resize detection with window following
- Uses AeroSpace-inspired event-driven approach
- Manages peer window relationships (no leader/follower hierarchy)

### Implementation Philosophy

The implementation follows research documented in:
- [Window Stacking Design](window-stacking-design.md) - Original design goals and approach
- [AeroSpace Approach to Window Following](../research/aerospace-approach-to-window-following.md) - Reliable drag detection strategy

#### Why Not AXObserver?
Traditional macOS window management relies on `AXObserver` notifications for tracking window movement. However, this approach has significant reliability issues:
- Only ~30% application compatibility
- Frequent notification failures
- Complex state management
- Performance overhead from continuous monitoring

#### AeroSpace-Inspired Solution
Instead, we use an event-driven polling approach:

1. **CGEvent Tap**: System-wide mouse event detection
2. **Interaction Detection**: Identify when user starts dragging or resizing any tracked window
3. **Timer-Based Polling**: Track active window position and size only during interactions (50ms intervals)
4. **Synchronous Following**: Move and resize all other windows to match the active window

This provides:
- 90%+ application compatibility
- Reliable cross-application window management
- Minimal CPU overhead (polling only during interactions)
- Low latency response (20fps during operations)

## Data Storage

### Project Schema Evolution
Stacked windows required updating the project schema from version 1 to version 2:

```swift
struct Project: Codable {
    let version: Int                    // Now 2
    // ... existing fields ...
    var stackedWindowsEnabled: Bool = false  // New field
}
```

### Migration Strategy
The implementation includes proper migration logic:
- Version 1 projects automatically upgrade to version 2
- `stackedWindowsEnabled` defaults to `false` for migrated projects
- Version 0 (legacy) projects also supported through fallback migration

This establishes a clean precedent for future schema upgrades.

## Implementation Details

### Window Positioning and Synchronization
When a taskspace is activated in stacked mode:

1. **Focus Target**: Bring target window to front using standard macOS APIs
2. **Get Bounds**: Retrieve target window position and size
3. **Position Others**: Move all other taskspace windows to exact same bounds
4. **Start Tracking**: Initialize interaction detection for all windows in the stack

### Interaction Following Process

```swift
// Simplified flow
func handleMouseEvent(type: CGEventType, event: CGEvent) {
    switch type {
    case .leftMouseDown:
        if trackedWindowIDs.contains(clickedWindow) {
            startTracking(activeWindow: clickedWindow) // Begin 50ms polling
        }
    case .leftMouseUp:
        stopActiveTracking()   // End polling
    }
}

func updateOtherWindows() {
    let positionDelta = currentPosition - lastPosition
    let hasResize = currentSize != lastSize
    
    for otherWindow in otherWindows {
        if hasMovement && hasResize {
            moveAndResizeWindow(otherWindow, positionDelta: positionDelta, newSize: currentSize)
        } else if hasMovement {
            moveWindow(otherWindow, by: positionDelta)
        } else if hasResize {
            resizeWindow(otherWindow, to: currentSize)
        }
    }
}
```

### Resource Management
- **Event Tap**: Created once, reused across all tracking sessions
- **Timer**: Only active during interaction operations (typically 1-3 seconds)
- **Cleanup**: Automatic cleanup when disabling stacked mode or closing projects
- **Memory**: Minimal overhead - just window ID tracking and position/size deltas

## Edge Cases and Limitations

### Current Limitations
- **IDE Windows Only**: Currently applies only to VSCode windows (taskspace windows)
- **Single Stack**: All taskspace windows form one stack (no multiple stacks)
- **Manual Recovery**: If windows get out of sync, switching taskspaces re-aligns them

### Handled Edge Cases
- **Window Closure**: Stale window references are automatically cleaned up
- **Project Switching**: Tracking stops when switching or closing projects
- **Mode Toggling**: Disabling stacked windows stops all tracking
- **Application Crashes**: Event tap and timers are properly cleaned up

## Future Enhancements

### Planned Improvements
- **Multiple Window Types**: Extend to terminal windows, browser windows, etc.
- **Multiple Stacks**: Support for organizing windows into different stacks
- **Visual Indicators**: Subtle visual cues showing stack membership
- **Keyboard Shortcuts**: Quick switching between stacked windows

### Performance Optimizations
- **Adaptive Polling**: Slower polling for small movements, faster for large movements
- **Movement Prediction**: Anticipate window movement for smoother following
- **Batch Updates**: Group multiple window moves into single operations

## Success Metrics

The stacked windows implementation is considered successful based on:

1. **Movement Coherence**: Dragging any window moves all others seamlessly ✅
2. **Resize Coherence**: Resizing any window resizes all others to match ✅
3. **Visual Isolation**: Only the active window is visible during normal operation ✅
4. **Reliable Switching**: Users can switch between windows without position drift ✅
5. **System Stability**: No performance impact or conflicts with macOS window management ✅
6. **Persistent Settings**: Per-project configuration survives app restarts ✅

## Conclusion

Stacked windows provides a clean, efficient way to manage multiple taskspace windows by creating the illusion of a single window that can be quickly switched between different contexts. The AeroSpace-inspired interaction detection ensures reliable window following across diverse applications while maintaining excellent performance characteristics.

The peer-based architecture allows any window in the stack to be the one the user interacts with, providing a natural and intuitive experience. Both drag and resize operations are synchronized across all windows in the stack, maintaining the illusion of working with a single window.

The implementation establishes solid patterns for future window management features and demonstrates how to build reliable cross-application window coordination on macOS.
