# Full-Screen Overlay Implementation

*Step-by-step implementation plan for replacing speech bubble panel with full-screen overlay approach*

## Context and Motivation

### Why This Change Is Needed
The current dock-activated interface (Phases 10-30 complete from [dock-activated-interface-implementation-plan.md](./dock-activated-interface-implementation-plan.md)) works functionally but suffers from unpredictable positioning. The speech bubble panel uses `estimateDockClickPosition()` which assumes dock is at screen center - this creates poor UX where the panel appears in seemingly random locations.

**Core Problem**: No reliable API exists to get actual dock icon coordinates. The current `DockPanel` approach uses complex heuristics that feel random to users.

**Solution**: Replace dock-relative positioning with a full-screen overlay system that's always centered and predictable, inspired by macOS Spotlight and Mission Control.

### Relationship to Existing Work
This implementation represents a refinement of **Phase 35 (UI Polish & Design Refinement)** from the dock-activated workflow. The underlying dock-activated system (XOR invariant, taskspace state management, IPC communication) remains unchanged - we're only replacing the presentation layer.

### Technical Architecture Shift
- **From**: `NSPanel` positioned relative to estimated dock location
- **To**: Full-screen `NSWindow` with centered overlay content
- **Result**: Eliminates positioning unpredictability while enabling rich animations

## Current State Analysis

### Existing Components (Reusable)
- **ProjectView (SwiftUI)**: Main taskspace display component → Can be reused in new overlay
- **TaskspaceCard (SwiftUI)**: Individual taskspace UI → Perfect for new grid layout
- **DockPanelManager**: Panel lifecycle management → Needs major refactoring for overlay approach
- **ProjectManager**: Core project/taskspace logic → Unchanged, pure reuse
- **IpcManager & ScreenshotManager**: Backend systems → Unchanged

### Components to Replace
- **DockPanel (NSPanel)**: Speech bubble with arrow → Replace with full-screen NSWindow
- **DockPanelContainerView**: Arrow drawing and positioning → Replace with centered layout
- **Panel positioning logic**: Dock-relative positioning → Replace with centered overlay

### Components to Create
- **FullScreenOverlayWindow (NSWindow)**: Main overlay window
- **OverlayAnimationController**: Manages contraction/expansion animations
- **ResponsiveGridLayout**: Dynamic taskspace grid sizing

## Implementation Phases

### Phase 1: Basic Full-Screen Overlay ⏳ **CURRENT PRIORITY**

**Objective**: Replace current panel with static full-screen overlay (no animation yet)

#### 1.1: Create FullScreenOverlayWindow
```swift
class FullScreenOverlayWindow: NSWindow {
    // Full-screen window with transparent background
    // 70% black overlay background
    // Centered content area for taskspaces
}
```

**Key features:**
- Full-screen coverage on main display
- Transparent window with semi-opaque background
- Click-outside-to-dismiss behavior
- Escape key handling

#### 1.2: Create OverlayContentView
```swift
class OverlayContentView: NSView {
    // Container for centered panel content
    // Handles backdrop dimming
    // Contains SwiftUI hosting view
}
```

**Layout structure:**
- Full-screen background with 70% black overlay
- Centered rectangle for taskspace content
- Proper margins and spacing

#### 1.3: Modify DockPanelManager
- Replace NSPanel creation with NSWindow
- Update positioning logic (center instead of dock-relative)
- Maintain existing API for compatibility
- Ensure ProjectView integration still works

**Success criteria for Phase 1:**
- [ ] Dock click shows full-screen dimmed overlay
- [ ] Taskspaces appear in centered panel
- [ ] Click outside dismisses overlay
- [ ] Escape key dismisses overlay
- [ ] All existing functionality preserved (taskspace clicks, project close, etc.)

### Phase 2: Responsive Layout System

**Objective**: Implement proper grid sizing based on content needs

#### 2.1: Text-Based Width Calculation
```swift
extension String {
    func widthForTaskspaceCard(font: NSFont) -> CGFloat {
        // Measure sample text: "Scotty, take us out here! Warp nine!"
        // Add padding for taskspace card chrome
        // Return consistent width for all cards
    }
}
```

#### 2.2: Dynamic Panel Sizing
```swift
struct OverlayLayoutCalculator {
    let screenSize: NSSize
    let columnCount: Int // Configurable 3-4
    let taskspaceCount: Int
    
    func calculatePanelSize() -> NSSize
    func shouldShowScrolling() -> Bool
    func rowCount() -> Int
}
```

#### 2.3: Grid Layout Integration
- Update ProjectView to use calculated dimensions
- Implement scrolling for >2 rows
- Ensure responsive behavior across screen sizes

**Success criteria for Phase 2:**
- [ ] Panel width adapts to taskspace content needs
- [ ] Column count is configurable
- [ ] Scrolling works for large taskspace counts
- [ ] Layout responsive to different screen sizes

### Phase 3: Core Animation Integration

**Objective**: Add smooth contraction/expansion animations

#### 3.1: Animation Controller
```swift
class OverlayAnimationController {
    func animateAppearance(overlay: FullScreenOverlayWindow)
    func animateDismissal(overlay: FullScreenOverlayWindow)
    
    // Manages scale transforms and timing
    // Coordinates background fade with panel scaling
}
```

#### 3.2: Scale-Based Transitions
- **Appearance**: Panel starts at screen size, contracts to center
- **Dismissal**: Panel expands from center to screen size
- **Timing**: Smooth, responsive animations (~0.3s duration)

#### 3.3: Visual Polish
- Easing curves for natural motion
- Coordinate background opacity with panel scaling
- Smooth 60fps performance

**Success criteria for Phase 3:**
- [ ] Smooth contraction animation on appearance
- [ ] Smooth expansion animation on dismissal
- [ ] 60fps performance with no dropped frames
- [ ] Natural, satisfying motion timing

### Phase 4: Advanced Features & Polish

**Objective**: Production-ready implementation with edge cases handled

#### 4.1: Multi-Screen Support
- Detect which screen contains dock
- Show overlay on appropriate screen
- Handle screen configuration changes

#### 4.2: Enhanced Interaction
- Keyboard shortcuts (⌘W to close, etc.)
- Trackpad gesture support for dismissal
- Improved focus handling

#### 4.3: Performance Optimization
- Efficient overlay creation/destruction
- Memory leak prevention
- Smooth animation even with many taskspaces

**Success criteria for Phase 4:**
- [ ] Works correctly on multi-monitor setups
- [ ] Keyboard shortcuts function properly
- [ ] No memory leaks or performance issues
- [ ] Handles edge cases gracefully

## Technical Architecture Changes

### File Structure Changes
```
Sources/Symposium/
├── FullScreenOverlay/
│   ├── FullScreenOverlayWindow.swift        # NEW
│   ├── OverlayContentView.swift            # NEW
│   ├── OverlayAnimationController.swift    # NEW
│   └── ResponsiveGridLayout.swift          # NEW
├── DockPanelManager.swift                  # MAJOR REFACTOR
├── ProjectView.swift                       # MINOR UPDATES (grid integration)
├── TaskspaceCard.swift                     # UNCHANGED
└── [other existing files]                 # UNCHANGED
```

### API Compatibility
Maintain existing DockPanelManager interface:
```swift
// Existing API preserved
func showPanel(with projectManager: ProjectManager, near point: NSPoint, onCloseProject: (() -> Void)?)
func hidePanel()
func togglePanel(...)

// Internal implementation completely changed
// External callers (AppDelegate, SplashView) unchanged
```

## Migration Strategy

### Backward Compatibility
- Keep existing API surface unchanged
- Maintain all current functionality
- Preserve user data and settings
- No breaking changes for existing projects

### Gradual Rollout
1. **Phase 1**: Replace backend, keep same functionality
2. **Phase 2**: Add layout improvements (user-visible enhancements)
3. **Phase 3**: Add animations (polish and delight)
4. **Phase 4**: Add advanced features (power user capabilities)

### Rollback Plan
- Feature flag for overlay vs panel approach
- Ability to revert to speech bubble panel if issues discovered
- Preserve old implementation until new one is battle-tested

## Testing Strategy

### Unit Tests
- Layout calculation logic
- Animation timing and curves
- Panel sizing edge cases
- Multi-screen scenarios

### Integration Tests
- Full dock-click workflow
- Project lifecycle with overlay
- Taskspace interaction through overlay
- Performance under load

### User Testing
- Discoverability of new interaction
- Intuitive dismissal methods
- Performance on different hardware
- Accessibility considerations

## Risk Mitigation

### Technical Risks
**Risk**: Full-screen overlay conflicts with other system UI  
**Mitigation**: Use appropriate window levels, test with various system states

**Risk**: Animation performance issues on older hardware  
**Mitigation**: Performance profiling, fallback to simple animations

**Risk**: Complex layout calculations causing sizing issues  
**Mitigation**: Extensive testing with edge cases, conservative defaults

### User Experience Risks
**Risk**: Overlay feels too aggressive/modal compared to panel  
**Mitigation**: User testing, possible hybrid approach

**Risk**: Loss of visual connection to dock interaction  
**Mitigation**: Clear animation suggesting origin from dock area

## Commit Strategy

### Phase 1 Commits
1. `feat: add FullScreenOverlayWindow foundation`
2. `feat: implement OverlayContentView with centered layout`
3. `refactor: update DockPanelManager for overlay approach`
4. `feat: add click-outside and escape key dismissal`
5. `test: verify overlay functionality matches existing panel`

### Phase 2 Commits
1. `feat: implement text-based taskspace width calculation`
2. `feat: add configurable column count for taskspace grid`
3. `feat: implement dynamic panel sizing and scrolling`
4. `feat: add responsive layout for different screen sizes`

### Phase 3 Commits
1. `feat: add OverlayAnimationController foundation`
2. `feat: implement contraction animation for panel appearance`
3. `feat: implement expansion animation for panel dismissal`
4. `polish: fine-tune animation timing and easing curves`

### Phase 4 Commits
1. `feat: add multi-screen support for overlay positioning`
2. `feat: implement keyboard shortcuts and gesture support`
3. `perf: optimize overlay performance for large taskspace counts`
4. `docs: update user documentation for new overlay interaction`

## Success Metrics

### Functional Requirements
- [ ] All existing dock panel functionality preserved
- [ ] Overlay appears on dock click consistently
- [ ] Panel content loads correctly in new layout
- [ ] Taskspace interaction works through overlay
- [ ] Project close functionality accessible

### Performance Requirements
- [ ] Overlay appears within 100ms of dock click
- [ ] Animations run at 60fps on supported hardware
- [ ] Memory usage stable across overlay open/close cycles
- [ ] No noticeable lag with 10+ taskspaces

### User Experience Requirements
- [ ] Interaction feels familiar and intuitive
- [ ] Clear visual feedback for all user actions
- [ ] Smooth, polished animations enhance rather than distract
- [ ] Consistent behavior across different screen configurations

Ready to begin Phase 1 implementation!