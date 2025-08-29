# Window Stacking Design

## Problem Statement

In Symposium we wish to create the illusion of there being only one taskspace visible at a time. We achieve this by creating a "stack" of windows where the leader taskspace is on-top and the others ("followers") are underneath, making them invisible. The problem is that when the leader is moved, we need to ensure that the followers move as well or the illustion will be ruined. This document explores our design for achieving this in a smooth fashion.

## Design Goals

1. **Single visible window**: Only the active ("leader") window should be visible at any time
2. **Cohesive movement**: When the leader window is moved, all stacked windows move together
3. **Reliable tracking**: Changes in window position should be detected promptly and accurately
4. **Smooth transitions**: Switching between windows in a stack should be fluid
5. **Minimal performance impact**: Window tracking should not significantly impact system resources

## Technical Approach

### Leader/Follower Architecture

Each window stack consists of:
- **Leader window**: The currently visible window that responds to user interaction
- **Follower windows**: Hidden windows that track the leader's position

Only one window per stack acts as the leader at any time. All follower windows are positioned slightly inside the leader window's bounds to ensure they remain hidden even during movement lag.

### Inset Positioning Strategy

Follower windows are positioned with a configurable inset relative to the leader window:

```
Leader window: (x, y, width, height)
Follower window: (x + inset, y + inset, width - 2*inset, height - 2*inset)

Default inset: 10% of window dimensions
```

**Example calculation:**
- Leader at (100, 100) with size 1000×800
- 10% inset = 100px horizontal, 80px vertical
- Follower at (150, 140) with size 900×720

This inset serves multiple purposes:
- **Lag compensation**: Even with 50-100ms notification delays, followers won't peek out during movement
- **Click protection**: Prevents accidental interaction with hidden follower windows
- **Visual clarity**: Makes the leader window unambiguously the active one

### Movement Tracking with kAXMovedNotification

Window position tracking uses the Accessibility API's notification system:

1. **Subscribe** to `kAXMovedNotification` for the current leader window
2. **Receive notification** when leader moves (50-100ms latency typical)
3. **Update all followers** to maintain relative position offset
4. **No subscriptions on followers** to avoid feedback loops

### Leader Election and Handoff

When switching the active window in a stack:

1. **Unsubscribe** from current leader's movement notifications
2. **Resize and reposition** old leader to follower dimensions/position
3. **Resize and reposition** new leader to leader dimensions/position
4. **Raise new leader** to top in window depth ordering
5. **Subscribe** to new leader's movement notifications

This subscription handoff pattern ensures:
- No cascading updates during programmatic window movement
- Clean separation between user-initiated and system-initiated position changes
- Reliable tracking of the active window only

## Implementation Details

### Configuration Options

| Parameter | Default | Range | Description |
|-----------|---------|--------|-------------|
| Inset percentage | 10% | 5-20% | Follower window inset as percentage of leader size |
| Minimum inset | 10px | 5-50px | Absolute minimum inset for very small windows |
| Maximum inset | 150px | 50-300px | Absolute maximum inset for very large windows |
| Notification timeout | 200ms | 100-500ms | Max wait time for position updates |

## Edge Cases and Mitigations

### Very Small Windows
- Apply minimum absolute inset (10px) regardless of percentage
- May result in minimal visual separation but preserves functionality

### Very Large Windows  
- Apply maximum absolute inset (150px) to avoid excessive unused space
- Maintains reasonable follower window usability

### Manual Follower Movement
- Followers moved manually (via Mission Control, etc.) are ignored
- Position will be corrected on next leader switch
- Alternative: Periodic position verification (future enhancement)

### System-Initiated Movement
- Display configuration changes may move all windows
- Leader movement notifications will trigger follower repositioning
- Natural recovery through normal tracking mechanism

### Application Misbehavior
- Some applications may resist programmatic repositioning
- Error handling should gracefully exclude problematic windows from stacks
- Logging available for debugging positioning failures

## User Interface Integration

### Stack Management Window
A separate monitoring window provides:
- List of active stacks with window counts
- Visual indication of current leader in each stack
- Click-to-switch functionality
- Future: Periodic thumbnail snapshots of stack contents

### Visual Indicators
Current approach: Separate monitoring window
Future possibilities:
- Wider background window creating subtle drop shadow
- Menu bar indicator with stack picker
- Dock integration with notification badges

## Performance Considerations

### Notification Efficiency
- Each stack requires one active notification subscription (leader only)
- System can handle 20-30 simultaneous subscriptions efficiently
- No polling required for position tracking

### Movement Latency
- 50-100ms typical delay between user drag and follower update
- Inset positioning makes this delay invisible to user
- Acceptable for intended use case of task-focused window management

### Memory Usage
- Minimal overhead: notification observers and window position tracking
- No additional window content rendering or capture required
- Scales linearly with number of active stacks

## Future Enhancements

### Animated Transitions
- Smooth resize/reposition animations during leader switches
- Configurable animation duration and easing
- May require Core Animation integration

### Advanced Visual Indicators  
- Semi-transparent follower window previews
- Stack depth indicators
- Drag handles or manipulation widgets

### Multi-Stack Management
- Named stacks with persistence
- Drag-and-drop between stacks
- Keyboard shortcuts for stack navigation

### Application Integration
- VS Code extension for automatic window stacking
- Terminal session integration
- WebSocket API for programmatic control

## Implementation Phases

### Phase 1: Core Functionality ✓
- Basic window stacking and switching
- Manual add/remove from stacks
- Simple position synchronization

### Phase 2: Movement Tracking (Current)
- Implement kAXMovedNotification system
- Add inset positioning for followers
- Create leader election and handoff logic

### Phase 3: Enhanced UX
- Stack monitoring window
- Configuration options
- Improved error handling and edge cases

### Phase 4: Advanced Features
- Animation system
- Multiple stack support
- External API integration

## Success Criteria

The window stacking implementation will be considered successful when:

1. **Movement coherence**: Dragging the leader window moves all followers seamlessly
2. **Visual isolation**: Only the leader window is visible during normal operation
3. **Reliable switching**: Users can switch between windows in a stack without position drift
4. **System stability**: No performance impact or conflicts with macOS window management
5. **Edge case handling**: Graceful behavior during display changes, app crashes, and unusual scenarios

This design provides a solid foundation for implementing true window stacking behavior while maintaining system compatibility and user experience quality.