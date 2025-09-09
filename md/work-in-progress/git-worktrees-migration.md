# Git Worktrees Migration Plan

## Overview

Replace the current git clone approach for taskspaces with git worktrees to improve disk usage, setup speed, and enable easier collaboration between taskspaces.

## Current Approach

```
project.symposium/
├── task-UUID-1/
│   └── repo-name/          # Full git clone (~50MB+ .git)
├── task-UUID-2/
│   └── repo-name/          # Another full git clone (~50MB+ .git)
└── task-UUID-3/
    └── repo-name/          # Yet another full git clone (~50MB+ .git)
```

## Proposed Approach

```
project.symposium/
├── .git/                   # Single bare repository (~50MB)
├── task-UUID-1/            # Git worktree on branch taskspace-UUID-1
├── task-UUID-2/            # Git worktree on branch taskspace-UUID-2
└── task-UUID-3/            # Git worktree on branch taskspace-UUID-3
```

## Benefits

- **Disk Space**: N × (working files) instead of N × (working files + .git)
- **Setup Speed**: `git worktree add` is much faster than `git clone`
- **Shared State**: All worktrees see the same remotes, branches, and git config
- **Easy Collaboration**: Can merge/cherry-pick between taskspace branches
- **Branch Visibility**: `git log --graph --all` shows all taskspace work

## Implementation Changes

### 1. ProjectManager.swift Changes

Replace the current clone logic in `createTaskspace()`:

**Current:**
```swift
let process = Process()
process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
process.arguments = ["clone", project.gitURL, cloneDir]
```

**New:**
```swift
// First taskspace: create bare repository
if !bareRepoExists {
    process.arguments = ["clone", "--bare", project.gitURL, "\(project.directoryPath)/.git"]
}

// All taskspaces: create worktree with unique branch
let branchName = "taskspace-\(taskspace.uuid)"
process.arguments = ["worktree", "add", taskspaceDir, "-b", branchName]
```

### 2. Directory Structure Updates

- Taskspaces will be created directly in `project.symposium/task-UUID/` (no nested repo directory)
- The bare `.git` repository lives at `project.symposium/.git`
- Update path resolution logic to account for flattened structure

### 3. Cleanup Considerations

- Need to handle `git worktree remove` when deleting taskspaces
- Consider `git worktree prune` for cleanup of stale worktree references

## Migration Strategy

1. **New projects**: Use worktree approach immediately
2. **Existing projects**: Could migrate by:
   - Converting existing clone to bare repo
   - Converting taskspace directories to worktrees
   - (Or just use new approach for new taskspaces)

## Future Enhancements

- Allow users to rename their `taskspace-UUID` branches to something meaningful
- Smart branch creation (detect if starting from specific commit/branch)
- UI to show branch relationships between taskspaces
