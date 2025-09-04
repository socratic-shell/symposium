# Window Per Project Architecture

## Current Problem

The current Symposium app uses a single main window that switches between different views:
- Project selection
- Settings/permissions
- Project view with taskspaces

This creates an unnatural user experience where:
- Only one project can be viewed at a time
- The window serves multiple purposes (setup, project management)
- No clear separation between "app configuration" and "project work"

## Proposed Architecture

### Two Window Types

#### 1. Splash/Setup Window
**Purpose**: App configuration and project selection
**Appears when**:
- No last project to restore
- Required permissions are missing
- User explicitly opens project selection

**Contains**:
- Permission checks and requests
- Settings interface
- Project selection/creation UI
- Agent configuration

**Behavior**:
- Shows on app startup when needed
- Closes automatically when project is selected
- Can be reopened via menu item
- Single instance only

#### 2. Project Window(s)
**Purpose**: Individual project management and taskspace orchestration
**Appears when**: Project is opened/created

**Contains**:
- Project-specific taskspace panels
- Window tiling controls
- Real-time taskspace screenshots
- Activity logs and progress indicators

**Behavior**:
- One window per project
- Multiple projects can be open simultaneously
- Window title shows project name
- Persists project state independently
- Proper window sizing (300-500px width, 400px+ height)

## Implementation Plan

### Phase 1: Window Scene Separation
- Create `SplashWindowGroup` scene in App.swift
- Create `ProjectWindowGroup` scene in App.swift
- Move appropriate views to each scene

### Phase 2: Window Management Logic
- App-level state to track open projects
- Startup logic to determine which windows to show
- Project opening/closing coordination

### Phase 3: State Management Refactor
- Move `ProjectManager` instances out of views
- App-level coordination between windows
- Proper window lifecycle management

### Phase 4: Multi-Project Support
- Support multiple simultaneous project windows
- Window restoration on app restart
- Menu items for window management

## Technical Details

### Window Scenes Structure
```swift
@main
struct SymposiumApp: App {
    var body: some Scene {
        // Splash/Setup window
        WindowGroup("setup") {
            SplashView()
        }
        .windowResizability(.contentSize)
        
        // Project windows (can have multiple)
        WindowGroup("project", for: ProjectIdentifier.self) { $projectId in
            ProjectWindow(projectId: projectId)
        }
        .windowResizability(.contentMinSize)
    }
}
```

### Window Coordination
- `AppCoordinator` class to manage window lifecycle
- Project opening triggers new project window
- Splash window closes when project selected
- Menu items to reopen splash or switch between projects

### Benefits
- **Natural UX**: Each project gets dedicated space
- **Multi-project workflow**: Work on multiple projects simultaneously  
- **Clear separation**: Setup vs. work contexts are distinct
- **Better window management**: Proper sizing and behavior per window type
- **Scalability**: Easy to add more project windows as needed

## Success Criteria
- [x] Splash window appears only when needed (no last project or config issues)
- [x] Project windows open independently for each project
- [x] Multiple projects can be open simultaneously
- [x] Window titles reflect project names
- [x] Proper window sizing constraints per window type
- [x] Clean window lifecycle (splash closes when project opens)
- [ ] Menu items for window management
