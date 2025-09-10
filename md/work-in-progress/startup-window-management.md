# Startup Window Management Implementation

This chapter tracks the implementation of the [Startup and Window Management](../design/startup-and-window-management.md) design document.

## Overview

We're transitioning from the current single-window splash system to a three-window architecture with a proper state machine. This involves:

1. **Project Metadata Migration** - Update project schema to include agent and version fields
2. **Window Architecture** - Create three distinct window types (Settings, Project Selection, Project Window)  
3. **State Machine** - Implement deterministic startup flow
4. **Migration Logic** - Handle existing projects gracefully

## Current Status: Phase 1 In Progress

### Phase 1: Project Metadata Migration ✅ Mostly Complete

**Goal**: Update the `Project` struct to match the target schema and handle migration of existing projects.

**Target Schema**:
```swift
struct Project: Codable, Identifiable {
    let version: Int = 1
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    let agent: String?        // Selected agent, nil if none
    let defaultBranch: String? // Default branch for new taskspaces, nil = auto-detect
    var taskspaces: [Taskspace] = []
    let createdAt: Date
}
```

**Completed**:
- ✅ Added version and agent fields to Project struct
- ✅ Implemented backward-compatible loading with ProjectV0 fallback
- ✅ Added migration logic that auto-saves upgraded projects
- ✅ Updated design document with defaultBranch field
- ✅ Added defaultBranch field to Project struct
- ✅ Updated ProjectManager.createProject() with agent and defaultBranch parameters
- ✅ Updated migration logic to set defaultBranch to nil (auto-detect)
- ✅ Added agent selection and Advanced Settings to project creation UI

**Remaining**:
- [ ] Update taskspace creation to use defaultBranch field (detect origin's default when nil)
- [ ] Test migration and new project creation

### Phase 2: Window Architecture (Planned)

- Create Settings window (separate from splash)
- Create Project Selection window 
- Create Project Window (main workspace)
- Update App.swift with three WindowGroups

### Phase 3: State Machine Implementation (Planned)

- Implement `appStart()` coordinator function
- Add window management helpers
- Implement state transitions and validation
- Handle error cases and edge conditions

## Implementation Notes

### Migration Strategy

When loading existing projects:
1. Try to decode with new schema first
2. If that fails, try legacy schema and add default values
3. Save migrated project back to disk with new schema

### Agent Handling

- Store agent as optional string (agent ID/name)
- Validate agent availability on project load
- Show warning in UI if selected agent is missing
- Allow project to function without agent (graceful degradation)

### Default Branch Handling

- Store defaultBranch as optional string (remote/branch reference)
- Support full remote/branch syntax (e.g., `origin/main`, `origin/develop`)
- Auto-detect origin's default branch when field is null/empty
- Use `git symbolic-ref refs/remotes/origin/HEAD` with fallback to `origin/main`
- New taskspaces start from specified or detected remote branch

## Next Steps

- [ ] Update taskspace creation logic to use project.defaultBranch field
- [ ] Test complete project creation, migration, and taskspace creation workflow
- [ ] Begin Phase 2: Window Architecture implementation
