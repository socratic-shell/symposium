# Repository Restructure: Socratic Shell + Symposium Split

## Overview

This document outlines the plan to reorganize the monorepo to clearly separate **Socratic Shell** (MCP server + VSCode extension) from **Symposium** (GUI app) while maintaining shared components and a unified build system.

## Current Structure Issues

1. **Naming confusion**: The VSCode extension is currently branded as "Symposium" but functionally it's the Socratic Shell IDE integration
2. **Mixed responsibilities**: Components for two distinct products are intermixed at the root level
3. **Unclear boundaries**: It's not obvious which files belong to which product

## Target Structure

```
# Root level
├── Cargo.toml          # Workspace coordination
├── md/                 # Project documentation (unified)
├── artwork/            # Logos, icons, branding assets
├── setup/              # Setup script (stays at root)
├── package.json        # If needed for coordination
└── README.md           # Project overview

socratic-shell/
├── socratic-shell/mcp-server/         # Rust MCP server
└── vscode-extension/   # VSCode extension (renamed from socratic-shell/vscode-extension)

symposium/
└── macos-app/          # macOS GUI app (from symposium/macos-app)
```

## Implementation Plan

### Phase 1: Directory Restructure and Cleanup

**Moves:**
- `socratic-shell/vscode-extension/` → `socratic-shell/vscode-extension/`
- `socratic-shell/mcp-server/` → `socratic-shell/mcp-server/`
- `symposium/macos-app/` → `symposium/macos-app/`
- Logo files → `artwork/`
- `create-app-icon.sh` → `artwork/` (will need path updates)

**Deletions:**
- `.socratic-shell/` directory (outdated)
- `bringing-it-all-together.md` (outdated)

### Phase 2: Rebranding and Configuration Updates

**VSCode Extension Rebranding:**
- Update `package.json`: name, displayName, description
- Change from "Symposium" to "Socratic Shell"
- Update all internal references and command names
- Update publisher if needed

**MCP Server Rebranding:**
- Rename crate from `mcp-server` to `socratic-shell-mcp` (or similar)
- Update `Cargo.toml` package name
- Update internal references and documentation

**Workspace Configuration:**
- Update root `Cargo.toml` workspace member paths
- Ensure all workspace dependencies still resolve correctly

**Setup Script Updates:**
- Update paths to reflect new structure
- Update any references to old crate names
- Test installation flow with new structure

### Phase 3: Documentation and Asset Updates

**Path Updates:**
- Update any hardcoded paths in documentation
- Update build scripts and references
- Fix `create-app-icon.sh` for new location in `artwork/`

**Documentation Reorganization:**
- Rename `./design/symposium-ref-system.md` to `./design/socratic-shell-ref-system.md`
- Reorganize "Design and implementation" section in `SUMMARY.md` into clear subsections:
  - **Socratic Shell**: MCP Server Tools, Reference System, Ask Socratic Shell, Code walkthroughs, Dialect language
  - **Symposium**: Startup and Window Management, Stacked Windows, Window Stacking Design
  - **Shared Architecture**: Implementation Overview, Guidance and Initialization, Taskspace Deletion, IPC/Daemon
- Update any references to old component names
- Leave Work in Progress section untouched (many files are outdated)

### Phase 4: Testing and Validation

**Build Verification:**
- Ensure `cargo build` works from root
- Verify VSCode extension builds correctly
- Test setup script with new structure

**Functionality Testing:**
- Verify MCP server starts correctly with new name
- Test VSCode extension loads and functions
- Ensure all inter-component communication still works

## Considerations

### Shared Assets
- Logos and branding assets will live in `artwork/` at root level
- Both products may share some visual assets but have distinct branding

### Build Coordination
- Root-level `Cargo.toml` maintains workspace coordination
- Each product can be built independently but also as part of unified build
- Setup script coordinates installation of both products

### Documentation Strategy
- Single `md/` directory maintains unified documentation
- Clear sections distinguish between Socratic Shell and Symposium features
- Shared architectural concepts remain in common documentation

## Migration Checklist

- [x] Create new directory structure
- [x] Move files to new locations
- [x] Delete outdated files and directories
- [x] Update VSCode extension branding
- [x] Rename MCP server crate
- [x] Update workspace configuration
- [x] Fix setup script paths and references
- [x] Update `create-app-icon.sh` location and paths
- [x] Rename `symposium-ref-system.md` to `socratic-shell-ref-system.md`
- [x] Reorganize SUMMARY.md "Design and implementation" section
- [x] Review and update documentation paths
- [x] Test complete build process
- [x] Verify all functionality works post-migration

## Rollback Plan

If issues arise during migration:
1. Git can restore the previous structure
2. Keep backup of critical configuration files
3. Test each phase incrementally to isolate issues
