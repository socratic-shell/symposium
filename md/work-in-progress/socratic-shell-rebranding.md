# Socratic Shell Rebranding Plan

## Overview

Complete the rebranding from "Symposium" to "Socratic Shell" throughout the codebase. This addresses the VSCode extension loading issue where view IDs don't match, and ensures consistent naming throughout the system.

## Current Status

**Fixed:**
- ✅ VSCode Extension: View container ID (`symposium` → `socratic-shell`)
- ✅ VSCode Extension: View ID (`symposium.walkthrough` → `socratic-shell.walkthrough`)

## Changes Required

### VSCode Extension - package.json
- [x] Command names: `symposium.*` → `socratic-shell.*`
  - [x] `symposium.showReview` → `socratic-shell.showReview`
  - [x] `symposium.copyReview` → `socratic-shell.copyReview`
  - [x] `symposium.logPIDs` → `socratic-shell.logPIDs`
  - [x] `symposium.showFileDiff` → `socratic-shell.showFileDiff`
  - [x] `symposium.addComment` → `socratic-shell.addComment`
  - [x] `symposium.addCommentReply` → `socratic-shell.addCommentReply`
  - [x] `symposium.addWalkthroughComment` → `socratic-shell.addWalkthroughComment`
  - [x] `symposium.toggleComments` → `socratic-shell.toggleComments`
  - [x] `symposium.toggleWindowTitle` → `socratic-shell.toggleWindowTitle`

### VSCode Extension - Source Code
- [x] Command registrations in `extension.ts` (already correct)
- [x] URL scheme: `symposium:` → `socratic-shell:`
  - [x] Renamed file: `symposiumUrl.ts` → `socraticShellUrl.ts`
  - [x] Updated all function names, types, and variable references
  - [x] Updated HTML data attributes: `data-symposium-url` → `data-socratic-shell-url`
- [x] Comment controller IDs: `symposium-walkthrough` → `socratic-shell-walkthrough`
- [x] IPC process name: `symposium-mcp` → `socratic-shell-mcp`

### MCP Server
- [x] Socket prefixes: `symposium-daemon` → `socratic-shell-daemon`
- [x] Directory structure: `.symposium` → `.socratic-shell`
- [x] Reference system: `<symposium-ref/>` → `<socratic-shell-ref/>`
- [x] Integration test socket names

## Do NOT Change

### VSCode Extension
- Repository URL (intentionally points to symposium repo)
- File references to `.symposium` directory (project structure)
- Window title prefixes `[symposium]` (user-facing feature)

## Implementation Strategy

### Phase 1: VSCode Extension Commands
1. Update package.json command names
2. Update command registrations in extension.ts
3. Test extension loading

### Phase 2: URL Scheme
1. Rename `symposiumUrl.ts` → `socratiShellUrl.ts`
2. Update all URL scheme references
3. Update file imports

### Phase 3: Comment Controllers & IPC
1. Update comment controller IDs
2. Update IPC process names
3. Test walkthrough functionality

### Phase 4: MCP Server
1. Update socket prefixes and daemon names
2. Update directory structure references
3. Update reference system
4. Update integration tests

### Phase 5: Testing
1. Verify VSCode extension loads without spinner
2. Verify walkthrough displays content
3. Verify IPC communication works
4. Run integration tests

## Risk Assessment

**Low Risk:**
- Command name changes (isolated to package.json and registration)
- Socket prefix changes (internal implementation)

**Medium Risk:**
- URL scheme changes (affects file navigation)
- Comment controller changes (affects walkthrough system)

**High Risk:**
- Directory structure changes (affects project detection)
- Reference system changes (affects cross-system communication)

## Testing Checklist

- [x] MCP server builds successfully (cargo check passes)
- [ ] VSCode extension loads without infinite spinner (requires TypeScript compilation)
- [ ] Walkthrough panel displays content
- [ ] Commands appear in command palette with correct names
- [ ] File navigation works with new URL scheme
- [ ] IPC communication between extension and MCP server
- [ ] Integration tests pass

## Status: MOSTLY COMPLETE

**Completed:**
- ✅ All command names updated
- ✅ All URL scheme references updated  
- ✅ All comment controller IDs updated
- ✅ All IPC process names updated
- ✅ All MCP server references updated
- ✅ MCP server builds successfully

**Next Steps:**
- VSCode extension needs TypeScript compilation to test fully
- Original loading issue should now be resolved (view IDs match)
- Ready for user testing

## Notes

This rebranding addresses the immediate issue where the VSCode extension shows a loading spinner because view IDs don't match between package.json and the webview provider registration.
