# Startup Window Management Implementation

This chapter tracks the implementation of the [Startup and Window Management](../design/startup-and-window-management.md) design document.

## Overview

We're transitioning from the current single-window splash system to a three-window architecture with a proper state machine. This involves:

1. **Project Metadata Migration** - Update project schema to include agent and version fields
2. **Window Architecture** - Create three distinct window types (Settings, Project Selection, Project Window)  
3. **State Machine** - Implement deterministic startup flow
4. **Migration Logic** - Handle existing projects gracefully

## Current Status: Planning

### Phase 1: Project Metadata Migration

**Goal**: Update the `Project` struct to match the target schema and handle migration of existing projects.

**Current Schema**:
```swift
struct Project: Codable, Identifiable {
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    var taskspaces: [Taskspace] = []
    let createdAt: Date
}
```

**Target Schema**:
```swift
struct Project: Codable, Identifiable {
    let version: Int = 1
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    let agent: String?  // Selected agent, nil if none
    var taskspaces: [Taskspace] = []
    let createdAt: Date
}
```

**Migration Requirements**:
- Add version field for future compatibility
- Add agent field for storing selected agent
- Handle loading projects without these fields (backward compatibility)
- Gracefully handle cases where stored agent is no longer available

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

## Next Steps

- [ ] Update Project struct with version and agent fields
- [ ] Implement backward-compatible loading logic
- [ ] Test migration with existing projects
- [ ] Update project creation to include agent selection
