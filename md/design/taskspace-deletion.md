# Taskspace Deletion System

## Overview

Taskspace deletion is a complex operation that involves multiple safety checks, git cleanup, and coordination between the Swift app and VSCode extension. This document describes the complete deletion workflow and the subtleties involved.

## Safety Checking System

### Branch Information Detection

Before showing the deletion dialog, the system computes fresh branch information:

```swift
func getTaskspaceBranchInfo(for taskspace: Taskspace) -> (branchName: String, isMerged: Bool, unmergedCommits: Int, hasUncommittedChanges: Bool)
```

**Key Components:**

1. **Branch Name Detection**: Uses `git branch --show-current` in the worktree directory
2. **Merge Status**: Uses `git merge-base --is-ancestor <branch> origin/main` 
3. **Unmerged Commits**: Uses `git rev-list --count <branch> --not origin/main`
4. **Uncommitted Changes**: Uses `git status --porcelain` (detects both staged and unstaged)

### Critical Git Command Fix

**Problem**: Initial implementation used `origin/origin/main` instead of `origin/main`
- `getBaseBranch()` returns `"origin/main"` (includes origin/ prefix)
- Git commands were adding another `origin/` prefix
- Result: `git merge-base --is-ancestor branch origin/origin/main` (invalid)

**Solution**: Use `baseBranch` directly without adding `origin/` prefix

### Timing of Branch Info Computation

**Key Insight**: Branch info is computed **when the dialog appears**, not when the app loads.

```swift
// In DeleteTaskspaceDialog
private var branchInfo: (branchName: String, isMerged: Bool, unmergedCommits: Int, hasUncommittedChanges: Bool) {
    projectManager.getTaskspaceBranchInfo(for: taskspace) // Fresh computation
}
```

**Why this matters:**
- User might make commits between app load and deletion attempt
- Stale branch info could show incorrect warnings
- Fresh computation ensures accurate safety warnings

## User Interface Warnings

### Warning Types

The system shows different warnings based on detected issues:

1. **Unmerged Commits**: "N commits from this branch do not appear in the main branch"
2. **Uncommitted Changes**: "This taskspace contains uncommitted changes" 
3. **Both Issues**: Shows both warnings separately
4. **Safe Case**: "This branch is safe to delete (no unmerged commits or uncommitted changes)"

### Default Toggle Behavior

The "Also delete branch" toggle defaults based on safety:

```swift
deleteBranch = (branchInfo.unmergedCommits == 0 && !branchInfo.hasUncommittedChanges)
```

- **Safe branches**: Toggle checked by default (encourage cleanup)
- **Risky branches**: Toggle unchecked by default (prevent accidental loss)

## Git Worktree Structure

### Directory Layout

```
project/
├── .git/                           # Bare repository
├── task-UUID1/
│   └── reponame/                   # Git worktree directory
│       ├── .git -> ../../.git/worktrees/...
│       └── [source files]
└── task-UUID2/
    └── reponame/                   # Another worktree
        └── [source files]
```

### Path Resolution Subtlety

**Critical**: Worktree path includes the repository name:

```swift
let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)  // /path/task-UUID
let repoName = extractRepoName(from: project.gitURL)                   // "symposium"  
let worktreeDir = "\(taskspaceDir)/\(repoName)"                       // /path/task-UUID/symposium
```

**Git commands must use `worktreeDir`, not `taskspaceDir`**

## Deletion Workflow

### Current Implementation

1. **Compute paths** (taskspaceDir, worktreeDir, branchName)
2. **Remove git worktree**: `git worktree remove <worktreeDir> --force`
3. **Fallback on failure**: `FileManager.removeItem(taskspaceDir)`
4. **Optionally delete branch**: `git branch -D <branchName>` (if user chose to)
5. **Update UI**: Remove taskspace from project model

### Git Command Execution Context

All git commands run from the **bare repository directory** (`project.directoryPath`):

```swift
process.currentDirectoryURL = URL(fileURLWithPath: project.directoryPath)
```

This is crucial because:
- Bare repository contains all git metadata
- Worktree directories only have symlinks to main .git
- Branch operations must run from the main repository

## Remaining Issues

### Git Cleanup Warnings

Despite path corrections, deletion still shows:
```
Warning: Failed to remove git worktree, falling back to directory removal
Warning: Failed to delete branch taskspace-UUID
```

**Hypothesis**: Timing issue or permission problem with git operations

**Debug approach**: Added logging to see actual paths being used:
```swift
Logger.shared.log("Attempting to remove worktree: \(worktreeDir) from directory: \(project.directoryPath)")
```

## Planned Enhancements

### Window Closure Coordination

**Problem**: VSCode windows remain open after taskspace deletion, showing "file not found" errors

**Solution**: Broadcast deletion intent before removing files:

```swift
// Before deletion
IpcManager.shared.broadcast(message: [
    "type": "taskspace_will_delete", 
    "taskspace_uuid": taskspace.id.uuidString
])

// VSCode extension closes window gracefully
vscode.commands.executeCommand('workbench.action.closeWindow')
```

## Architecture Insights

### Why This System is Complex

1. **Git Worktree Management**: Multiple worktrees sharing one repository
2. **Safety vs Convenience**: Balance between preventing data loss and smooth UX  
3. **Timing Dependencies**: Fresh data computation vs performance
4. **Cross-Process Coordination**: Swift app + VSCode extension + git commands
5. **Error Recovery**: Graceful fallbacks when git operations fail

### Design Principles

1. **Safety First**: Always warn about potential data loss
2. **Fresh Data**: Compute branch info when needed, not when cached
3. **Clear Communication**: Specific warnings for different risk types
4. **Graceful Degradation**: Fallback to directory removal if git fails
5. **User Control**: Let users choose branch deletion behavior

## Testing Scenarios

### Test Cases to Verify

1. **Clean branch**: No commits, no changes → Green "safe to delete"
2. **Unmerged commits**: Commits not in main → Orange warning + count
3. **Uncommitted changes**: Modified files → Orange "uncommitted changes" warning  
4. **Both issues**: Show both warnings separately
5. **Git cleanup**: Verify worktree and branch removal work without warnings
6. **Window closure**: VSCode windows close gracefully during deletion

### Edge Cases

1. **Detached HEAD**: How does branch detection work?
2. **Merge conflicts**: What if worktree has unresolved conflicts?
3. **Permission issues**: What if git commands fail due to permissions?
4. **Concurrent access**: What if multiple processes access the same worktree?

## Conclusion

The taskspace deletion system demonstrates the complexity of managing git worktrees safely while providing a smooth user experience. The key insights are:

1. **Fresh computation** of branch info prevents stale warnings
2. **Correct path resolution** is critical for git operations  
3. **Separate warnings** for different risk types improve user understanding
4. **Graceful fallbacks** ensure deletion works even when git operations fail

The system successfully prevents accidental data loss while maintaining usability.
