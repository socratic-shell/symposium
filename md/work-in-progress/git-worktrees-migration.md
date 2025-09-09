# Git Worktrees Migration Plan

## Overview

Complete the git worktrees implementation for taskspaces, including proper cleanup, branch management, and UI integration.

## Current Status

The basic worktree creation is implemented, but several key features are missing:
- Proper worktree cleanup on deletion
- Branch naming and renaming capabilities  
- UI integration for branch management
- Improved agent workflow for collaborative naming

## Directory Structure (Confirmed)

```
project.symposium/
├── .git/                   # Single bare repository
├── task-UUID-1/
│   └── project-name/       # Git worktree on branch taskspace-UUID-1
├── task-UUID-2/
│   └── project-name/       # Git worktree on branch taskspace-UUID-2
└── task-UUID-3/
    └── project-name/       # Git worktree on branch taskspace-UUID-3
```

**Note**: We maintain the `task-UUID/project-name/` nested structure, not a flattened approach.

## Required Implementation Work

### 1. Worktree Cleanup (Backend)
- ✅ Partially done: Basic worktree creation exists
- ❌ **TODO**: Add `git worktree remove` to `deleteTaskspace()`
- ❌ **TODO**: Offer user choice to delete git branch or keep it
- ❌ **TODO**: Handle cleanup gracefully with fallbacks

### 2. Branch Management (MCP Tool)
- ❌ **TODO**: Extend `update_taskspace` tool with optional `branch_name` parameter
- ❌ **TODO**: When `branch_name` provided, rename current git branch with `git branch -m`
- ❌ **TODO**: Update tool to support the new collaborative naming workflow

### 3. Remote Management (Git Integration)
- ❌ **TODO**: UI to configure user's push remote (typically their GitHub fork)
- ❌ **TODO**: Auto-detect GitHub fork when creating project (check if `username/repo-name` exists)
- ❌ **TODO**: Set up smart defaults for `git config push.default` and remote configuration
- ❌ **TODO**: Allow users to modify remote configuration through UI

### 4. UI Integration (Frontend)
- ❌ **TODO**: Display git branch name in project window alongside taskspace name/description
- ❌ **TODO**: Make branch name editable in UI
- ❌ **TODO**: When user edits branch name in UI, call backend to rename git branch
- ❌ **TODO**: Show branch name in taskspace cards/panels
- ❌ **TODO**: Add remote configuration UI in project settings

### 5. Improved Agent Workflow (Instructions)
- ❌ **TODO**: Update default initial prompt to encourage user discussion before naming
- ❌ **TODO**: Guide agents to propose name, description, and branch name together
- ❌ **TODO**: Encourage "Got it! Here's my proposal..." pattern

## Proposed Agent Workflow

**Current problematic flow:**
1. Agent immediately picks generic name like "Unnamed taskspace"
2. Agent updates without user input
3. User has to correct later

**New collaborative flow:**
1. Agent starts with instructions emphasizing user discussion
2. Agent talks with user to understand the work
3. Agent proposes: "Got it! Here's my proposal for a taskspace description: [name], [description], and I suggest we call the git branch [branch-name]. Does that work for you?"
4. If user agrees, agent calls `update_taskspace` with all three parameters

## Implementation Priority

1. **MCP Tool Enhancement** - Add branch naming to `update_taskspace`
2. **Backend Cleanup** - Proper worktree removal with user choice
3. **Remote Management** - Fork detection and push remote configuration
4. **UI Integration** - Display and edit branch names, remote settings
5. **Agent Instructions** - Updated initial prompt and workflow

## Benefits

- **Proper Git Hygiene**: Clean worktree and branch management
- **User Control**: Choice over branch deletion and naming
- **Fork Integration**: Seamless GitHub fork workflow with smart defaults
- **UI/Git Sync**: Branch names and remotes stay consistent between UI and git
- **Collaborative Naming**: Agents discuss with users before naming
- **Better Organization**: Meaningful branch names reflect taskspace purpose
