# Project Window Visual Design

*Centered panel interface for dock-activated taskspace management*

## Overview

Symposium's project interface uses a centered panel system that provides focused, predictable interaction with taskspace management. When users click the Symposium dock icon, a clean panel appears at the center of the screen, displaying all taskspaces in an intelligent grid layout that adapts to screen size and content requirements.

## User Experience

### Interaction Model
- **Dock activation**: Clicking the Symposium dock icon reveals the centered project panel
- **Centered positioning**: Panel appears at screen center for consistent, predictable placement
- **Grid-based layout**: Taskspaces arranged in responsive rows and columns for efficient browsing
- **Persistent interaction**: Panel stays visible for browsing, expanding, and managing taskspaces

### Interface Behavior
- **Browse mode**: Grid layout showing all taskspaces for quick overview and selection
- **Detail mode**: Individual taskspaces can expand to fill panel for detailed log viewing and management
- **Smart dismissal**: Panel persists for management tasks but disappears when user engages with taskspaces (launch/focus VSCode)
- **Future extensibility**: Pin option planned for persistent sidebar mode
- **Responsive design**: Adapts to different screen sizes while maintaining consistent taskspace sizing

## Layout System

### Responsive Grid Layout

**Taskspace Width Calculation (TW):**
The system calculates optimal taskspace width based on content requirements:

1. **Screenshot width**: Fixed at 120px for consistent thumbnail sizing
2. **Sample text width**: Measured against typical log messages ("Implementing authentication system...")
3. **Padding**: Margins and spacing between elements
4. **Result**: `TW = screenshot_width + text_width + padding`

**Panel Width Constraint System (PW):**
Panel width follows a prioritized constraint chain:

1. **Ideal width**: `4 * TW` (target 4 taskspaces per row)
2. **Screen constraint**: `min(ideal_width, 0.75 * screen_width)` (max 3/4 screen width)
3. **Minimum viable**: `max(constrained_width, 1 * TW)` (always fit at least one taskspace)
4. **Hard limit**: `min(final_width, screen_width)` (cannot exceed screen)

**Grid Layout Logic:**
```
taskspaces_per_row = floor(panel_width / taskspace_width)
num_rows = ceil(total_taskspaces / taskspaces_per_row)
panel_height = num_visible_rows * taskspace_height + header_height
```

**Overflow Management:**
- **Vertical scrolling**: Users scroll through rows when taskspace count exceeds visible area
- **Row-based navigation**: Scroll by complete rows for predictable navigation
- **Expand mode**: Selected taskspace fills entire panel for detailed log viewing

### Interaction Modes

The interface supports two primary interaction modes:

**Grid Mode (Default):**
```
┌─────────────────────────────────────────┐
│ Project: MyApp                 [📌] [X] │ ← Project header with pin option
│ ─────────────────────────────────────── │
│                                         │
│ [Taskspace 1] [Taskspace 2] [Task 3]   │ ← Primary row (up to 4 per row)
│                                         │
│ [Taskspace 4] [Taskspace 5] [Task 6]   │ ← Secondary row
│                                         │
│ ┌─────────────────────────────────────┐ │ ← Scrollable area
│ │ [Additional taskspaces...]          │ │   (when >2 rows)
│ └─────────────────────────────────────┘ │
│                                         │
│ [+] New Taskspace                       │
└─────────────────────────────────────────┘
```

**Detail Mode (Expanded Taskspace):**
```
┌─────────────────────────────────────────┐
│ Project: MyApp > Taskspace Name    [↩] │ ← Breadcrumb navigation
│ ─────────────────────────────────────── │
│                                         │
│ ┌─────┐ Implementing auth system        │ ← Expanded taskspace header
│ │ 📸  │ Status: Active                  │
│ └─────┘ [Focus Window] [⚙Settings]     │
│                                         │
│ Recent Activity:                        │ ← Scrollable log area
│ ┌─────────────────────────────────────┐ │
│ │ • Created JWT token validation      │ │
│ │ • Added middleware for auth         │ │
│ │ • Fixed session timeout bug        │ │
│ │ • [... 47 more entries]            │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Information Architecture:**
- **Project header**: Shows active project with pin option (future) and close control
- **Grid layout**: Taskspaces arranged in calculated columns with consistent spacing
- **Expand functionality**: Individual taskspaces can fill panel for detailed interaction
- **Navigation**: Clear breadcrumb system for moving between grid and detail modes

## Panel Dismissal Behavior

### Smart Dismissal System
The panel uses intelligent dismissal logic that distinguishes between management tasks and taskspace engagement:

**Panel Persists For:**
- Browsing taskspaces in grid mode
- Expanding taskspaces to detail mode
- Scrolling through taskspace logs
- Clicking taskspace management buttons (settings, etc.)
- Creating new taskspaces
- General panel interactions

**Panel Dismisses For:**
- Clicking taskspace to launch VSCode (dormant → active)
- Clicking taskspace to focus VSCode window (active taskspace engagement)
- Clicking outside panel area
- Pressing Escape key
- Closing project via [X] button

### Future Pin Functionality
**Planned Enhancement:**
- **Pin icon** in panel header toggles "always visible" mode
- **Pinned behavior**: Panel remains visible even during taskspace engagement
- **Sidebar mode**: Pinned panel can optionally dock to screen edge
- **User preference**: Remember pin state across sessions

## Visual Design

### Panel Architecture
The interface uses a clean, focused panel design built on NSPanel architecture:

- **Centered positioning**: Panel always appears at screen center for consistent, predictable placement
- **Clean design**: No background overlay or full-screen dimming
- **System integration**: Built on NSPanel with proper focus management and system behavior
- **Visual clarity**: Clean separation between taskspaces with thoughtful use of whitespace

### Animation System (Future Phase)
**Planned Enhancement:**
- **Smooth appearance**: Panel materializes at center with subtle scale/fade animation
- **Graceful dismissal**: Panel disappears with coordinated animation
- **Internal transitions**: Smooth expand/collapse for taskspace detail mode
- **Performance focused**: Lightweight animations that enhance rather than distract

## Technical Implementation

### Panel Architecture
The interface maintains NSPanel architecture with centered positioning and responsive layout:

- **NSPanel foundation**: Built on existing panel system with proper focus management
- **Centered positioning**: Screen center positioning eliminates unpredictable placement
- **Grid layout system**: SwiftUI-based responsive grid with calculated dimensions
- **Performance optimization**: Efficient rendering and memory management for responsive interaction

### Integration Points
- **Existing codebase**: Reuses current ProjectView, TaskspaceCard, and management systems
- **Dock interaction**: Maintains existing dock click detection and activation
- **System events**: Proper handling of screen changes and focus transitions
- **Multi-display**: Centers on screen containing dock (future enhancement)

## Design Philosophy

The centered panel approach embodies several key design principles:

**Predictability**: Consistent, centered positioning provides reliable user experience across different screen configurations

**Efficiency**: Intelligent grid layout system maximizes taskspace visibility while adapting to screen constraints and content requirements  

**Contextuality**: Smart dismissal behavior keeps panel available for management tasks while clearing the way for actual work

**Scalability**: Responsive layout system gracefully handles projects with varying taskspace counts, from single taskspace to dozens

**Extensibility**: Architecture supports future enhancements (animations, pin functionality) without breaking existing workflows

**Familiarity**: Builds on proven NSPanel foundation while providing consistent, predictable positioning

This design creates an intuitive, reliable interface that provides consistent taskspace management while adapting intelligently to different screen sizes and project scales.