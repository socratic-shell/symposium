# Symposium Panel Prototype

**Status**: Planning  
**Started**: 2025-09-02  
**Goal**: Build first working prototype of Symposium panel interface with settings dialog, agentspace management, and basic window tiling

## Overview

Transform the current window management testing interface into the actual Symposium panel as specified in [Interface plan](../design/interface-plan.md). This is the core user-facing component that orchestrates AI agent workspaces.

## Current State

- Basic Swift app structure exists with window management testing interface
- MCP server implemented and functional
- Architecture well-documented
- Need to implement actual panel interface per specification

## Key Components to Build

### 1. Settings Dialog
- Accessibility permission checking and request flow
- Screen recording permission handling  
- IDE detection and extension status checking
- Agent tool selection (Claude Code vs Q CLI)
- Communication preference (integrated terminal vs separate)

### 2. Main Panel Interface
- Tiling configuration buttons at top
- Agentspace list with screenshots and logs
- Progress indicators from `log_progress` MCP calls
- Attention management from `signal_user` calls

### 3. Dock Integration
- Badge showing active agentspaces + attention requests
- Panel appears on dock icon click

### 4. IPC Communication
- Unix socket connection to daemon
- Message handling for agentspace updates
- Screenshot capture coordination

## Technical Approach

Starting with UI mockup using static data, then adding real functionality:

1. **Phase 1**: Settings dialog with permission flows
2. **Phase 2**: Main panel layout with mock agentspace data  
3. **Phase 3**: IPC integration for real agentspace communication
4. **Phase 4**: Screenshot capture and window tiling

## Open Questions

- Should we implement ScreenCaptureKit integration early or use placeholder images initially?
- How to handle the transition from testing interface to production interface?
- What's the best way to structure SwiftUI views for maintainability?

## Next Steps

- [ ] Analyze current ContentView.swift structure
- [ ] Design SwiftUI component hierarchy
- [ ] Create SettingsView.swift with permission checking
- [ ] Build basic panel layout with mock data
