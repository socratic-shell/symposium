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
2. **Sample text width**: Measured against typical log messages ("Captain, we're getting mysterious sensor readings")
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

**Three-Area Layout Structure:**
The panel uses a consistent three-area layout that optimizes both usability and visual hierarchy:

1. **Fixed header** (top): Project information, controls, never scrolls
2. **Scrollable content** (middle): Dynamic grid of taskspaces, expands/contracts as needed  
3. **Fixed footer** (bottom): Primary actions, always accessible

**Overflow Management:**
- **Vertical scrolling**: Users scroll through rows when taskspace count exceeds visible area
- **Row-based navigation**: Scroll by complete rows for predictable navigation
- **Expand mode**: Selected taskspace fills entire content area for detailed log viewing

### Interaction Modes

The interface supports two primary interaction modes:

**Grid Mode (Default):**
```
┌─────────────────────────────────────────┐
│ Project: MyApp                 [📌] [X] │ ← Header: project info + controls
│ ─────────────────────────────────────── │
│                                         │
│ [Taskspace 1] [Taskspace 2] [Task 3]   │ ← Grid: primary row (up to 4 per row)
│                                         │
│ [Taskspace 4] [Taskspace 5] [Task 6]   │ ← Grid: secondary row
│                                         │
│ ┌─────────────────────────────────────┐ │ ← Grid: scrollable area
│ │ [Additional taskspaces...]          │ │   (when >2 rows visible)
│ └─────────────────────────────────────┘ │
│                                         │
│ ─────────────────────────────────────── │
│ [+] New Taskspace                       │ ← Footer: always-visible actions
└─────────────────────────────────────────┘
```

**Detail Mode (Expanded Taskspace):**
```
┌─────────────────────────────────────────┐
│ Project: MyApp > Taskspace Name    [↩] │ ← Breadcrumb navigation
│ ─────────────────────────────────────── │
│                                         │
│ ┌─────┐ Enterprise Security Systems     │ ← Expanded taskspace header
│ │ 📸  │ Status: Active                  │
│ └─────┘ [Focus Window] [⚙Settings]     │
│                                         │
│ Recent Activity:                        │ ← Scrollable log area
│ ┌─────────────────────────────────────┐ │
│ │ • Red alert! Klingon vessel detected│ │
│ │ • Shields at maximum, Captain!      │ │
│ │ • Scotty reports: "She cannae take  │ │
│ │   much more of this, Captain!"      │ │
│ │ • [... 47 more entries]            │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Information Architecture:**
- **Fixed header**: Shows active project with pin option (future) and close control
- **Scrollable grid**: Taskspaces arranged in calculated columns with consistent spacing
- **Fixed footer**: Always-visible actions (New Taskspace, future additional controls)
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

## Technical Implementation

- **NSPanel foundation**: Centered at screen center, no dock-relative positioning
- **Three-area layout**: Fixed header, scrollable grid content, fixed footer
- **Responsive sizing**: Panel width calculated using TW constraint chain
- **Grid layout**: Calculated columns based on screen width and taskspace count