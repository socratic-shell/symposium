# Fresh Context Summary

## What We Just Fixed

We completed a major architectural refactor to implement **single source of truth** pattern for ProjectManager instances. This was done to fix duplicate IpcManager issues that were causing cleanup problems.

### Key Changes Made

1. **Single Source of Truth**: `AppDelegate.currentProjectManager` is now the ONLY owner of ProjectManager
2. **Observer Pattern**: All views use `@EnvironmentObject var appDelegate: AppDelegate` instead of direct ProjectManager references
3. **Graceful Degradation**: Views handle `nil` ProjectManager by showing "No project selected"
4. **Clean Callbacks**: ProjectSelectionView uses callbacks instead of direct settings modification

### Architecture Before vs After

```swift
// ❌ BEFORE: Multiple references, cleanup issues
struct ProjectView: View {
    let projectManager: ProjectManager  // Duplicate reference!
    @ObservedObject var ipcManager: IpcManager  // Another reference!
}

// ✅ AFTER: Single source of truth
struct ProjectView: View {
    @EnvironmentObject var appDelegate: AppDelegate
    
    var body: some View {
        if let projectManager = appDelegate.currentProjectManager {
            // Use projectManager.mcpStatus instead of direct ipcManager
        } else {
            Text("No project selected")
        }
    }
}
```

### What Should Work Now

- **Clean Lifecycle**: Setting `appDelegate.currentProjectManager = nil` should properly clean up ALL references
- **No Duplicate IpcManagers**: Only one IpcManager instance should exist at a time
- **Proper Window Close**: Project path persists on app quit, clears on explicit window close
- **Automatic UI Updates**: All views automatically update when ProjectManager changes

### Current Status

- ✅ All code builds successfully
- ✅ Architecture refactor complete
- ✅ Documentation updated
- 🔄 **NEXT**: Test if duplicate IpcManager issue is resolved

### Key Files Modified

- `App.swift`: Updated to use single source of truth
- `ProjectView.swift`: Complete refactor to use AppDelegate
- `ProjectWindow.swift`: Updated to use AppDelegate
- `ProjectSelectionView.swift`: Uses callbacks instead of direct modification
- Documentation updated in `md/design/`

### Debugging Reference

Each ProjectManager logs its lifecycle:
- `ProjectManager[X]: Initializing` on creation  
- `ProjectManager[X]: Cleaning up` on deallocation
- `IpcManager[Y]: Cleaning up - terminating client process` on IPC cleanup

If you see multiple active instances, there's still a reference leak somewhere.

### Testing Needed

1. Open project → close window → check logs for proper cleanup
2. Verify only one IpcManager instance exists at a time
3. Test project selection flow works correctly
4. Verify project path persistence works as expected
