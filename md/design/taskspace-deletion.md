# Taskspace Deletion System

## Overview

Taskspace deletion is a complex operation that involves multiple safety checks, git cleanup, and coordination between the Swift app and VSCode extension. This document describes the architectural design and key insights.

## System Architecture

### Dialog Confirmation Flow

{RFD:taskspace-deletion-dialog-confirmation}

The deletion system now implements proper dialog confirmation to ensure agents receive accurate feedback:

**Previous Flow (Problematic)**:
1. Agent requests deletion → Immediate "success" response → UI dialog shown
2. User could cancel, but agent already thought deletion succeeded

**New Flow (Fixed)**:
1. Agent requests deletion → No immediate response → UI dialog shown  
2. User confirms → Actual deletion → Success response to agent
3. User cancels → Error response to agent ("Taskspace deletion was cancelled by user")

**Key Implementation**: The `MessageHandlingResult::pending` case allows the IPC system to defer responses until user interaction completes.

### Safety-First Design

The deletion system prioritizes preventing data loss through a multi-layered safety approach:

1. **Fresh Branch Analysis**: Computes git status when the deletion dialog opens (not when cached)
2. **Risk-Based Warnings**: Shows specific warnings for different types of uncommitted work
3. **Smart Defaults**: Auto-configures branch deletion toggle based on detected risks
4. **Graceful Fallbacks**: Continues deletion even if git operations fail

**Key Insight**: Branch information must be computed fresh when the dialog appears, not when the app loads, because users may make commits between app startup and deletion attempts.

### Cross-Process Coordination

The system coordinates between multiple processes:
- **Swift App**: Manages deletion workflow and safety checks
- **VSCode Extension**: Receives deletion broadcasts and closes windows gracefully  
- **Git Commands**: Handle worktree and branch cleanup operations

**Planned Enhancement**: Broadcast `taskspace_will_delete` messages before file removal to allow VSCode windows to close gracefully, preventing "file not found" errors.

## Git Worktree Integration

### Architectural Constraints

The system works within git worktree constraints:
- **Bare Repository**: All git operations must run from the main repository directory
- **Worktree Paths**: Include repository name (e.g., `task-UUID/reponame/`)
- **Shared Metadata**: Multiple worktrees share one `.git` directory

**Critical Design Decision**: All git commands execute from `project.directoryPath` (bare repo) rather than individual worktree directories, because worktrees only contain symlinks to the main git metadata.

### Directory Structure
```
project/
├── .git/                    # Bare repository (command execution context)
├── task-UUID1/
│   └── reponame/           # Git worktree (target for removal)
└── task-UUID2/
    └── reponame/           # Another worktree
```

## Design Principles

1. **Safety First**: Always warn about potential data loss before proceeding
2. **Accurate Agent Feedback**: Only respond to agents after user makes actual decision
3. **Fresh Data**: Compute branch info when needed, not when cached  
4. **Clear Communication**: Provide specific warnings for different risk types
5. **Graceful Degradation**: Continue deletion even when git operations fail
6. **User Control**: Let users choose branch deletion behavior based on clear information

## IPC Message Flow

{RFD:taskspace-deletion-dialog-confirmation}

### Deferred Response Pattern

The `delete_taskspace` IPC message uses a deferred response pattern:

1. **Request Received**: `handleDeleteTaskspace` stores the message ID and returns `.pending`
2. **No Immediate Response**: IPC manager doesn't send response yet
3. **Dialog Interaction**: User confirms or cancels in UI
4. **Deferred Response**: Appropriate success/error response sent based on user choice

This ensures the MCP server and agent receive accurate information about whether the deletion actually occurred.

## Complexity Drivers

### Why This System is Complex

1. **Git Worktree Management**: Multiple worktrees sharing one repository with complex path relationships
2. **Safety vs Convenience**: Balance between preventing data loss and smooth user experience
3. **Timing Dependencies**: Fresh computation requirements vs performance considerations  
4. **Cross-Process Coordination**: Swift app + VSCode extension + git subprocess coordination
5. **Error Recovery**: Graceful fallbacks when git operations fail due to various reasons

### Key Architectural Insights

1. **Fresh computation** of branch info prevents stale warnings that could mislead users
2. **Correct path resolution** is critical - git commands must target actual worktree paths
3. **Separate warning types** improve user understanding of different risks
4. **Execution context matters** - git commands must run from bare repository directory

## Testing Strategy

### Critical Test Scenarios

1. **Clean State**: No commits, no changes → Should show "safe to delete" 
2. **Unmerged Work**: Commits not in main → Should warn with commit count
3. **Uncommitted Work**: Modified files → Should warn about uncommitted changes
4. **Mixed State**: Both unmerged and uncommitted → Should show both warnings
5. **Git Operations**: Verify worktree and branch removal work without errors
6. **Window Coordination**: VSCode windows should close gracefully during deletion

### Edge Cases to Consider

1. **Detached HEAD**: How does branch detection behave?
2. **Merge Conflicts**: What happens with unresolved conflicts in worktree?
3. **Permission Issues**: How does system handle git command failures?
4. **Concurrent Access**: What if multiple processes access same worktree?
5. **Network Issues**: How does remote branch checking handle connectivity problems?

## Implementation References

**Key Methods** (see code comments for implementation details):
- `ProjectManager.getTaskspaceBranchInfo()` - Branch safety analysis
- `ProjectManager.deleteTaskspace()` - Main deletion workflow  
- `DeleteTaskspaceDialog` - UI warning logic and user interaction

**Critical Path Resolution** (see `deleteTaskspace()` comments):
- Worktree path calculation and git command targeting
- Execution context setup for git operations

**Safety Checking** (see `getTaskspaceBranchInfo()` comments):
- Git command details and error handling
- Fresh computation timing and rationale
