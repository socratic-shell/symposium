# WIP vs Design Section Overlap Analysis

## Summary of Findings

The remaining WIP chapters contain significant overlap with the Design section, but also some unique content that may be worth preserving or consolidating.

## Detailed Comparison

### 1. Big Picture Plans (`work-in-progress/big-picture.md`)
**Content**: High-level roadmap, app startup experience, project creation flow
**Overlap with Design**: 
- Some overlap with Implementation Overview (system architecture concepts)
- Startup flow overlaps with Startup and Window Management
**Unique Value**: 
- Roadmap/planning information not covered in Design
- High-level user experience flows
**Recommendation**: Keep as planning document, but consider moving completed items to Design

### 2. MVP README (`work-in-progress/mvp/README.md`)
**Content**: Installation workflow, permission granting, project selection
**Overlap with Design**:
- Installation overlaps with setup.md
- Permission granting overlaps with Startup and Window Management
- Project structure overlaps with Implementation Overview
**Unique Value**:
- Specific MVP scope definition
- Detailed installation steps
**Recommendation**: Content should be consolidated into Design section or removed if superseded

### 3. Taskspace Bootup Flow (`work-in-progress/mvp/taskspace-bootup-flow.md`)
**Content**: VSCode extension auto-detection, agent launching, MCP coordination
**Overlap with Design**:
- Significant overlap with MCP Server Tools and Taskspace Orchestration Tools
- Some overlap with Implementation Overview (IPC communication)
**Unique Value**:
- Detailed sequence diagram of bootup process
- Specific VSCode integration details
**Recommendation**: Should be moved to Design section under MCP tools or consolidated

### 4. Window Registration Design (`work-in-progress/mvp/window-registration-design.md`)
**Content**: VSCode window correlation, title handshake mechanism
**Overlap with Design**:
- Overlaps with Startup and Window Management
- Related to Implementation Overview (window management)
**Unique Value**:
- Detailed technical solution for window correlation problem
- Specific implementation approach
**Recommendation**: Should be moved to Design section - this is implemented functionality

### 5. Window Per Project (`work-in-progress/mvp/window-per-project.md`)
**Content**: Two window types (splash/setup vs project), architecture separation
**Overlap with Design**:
- Significant overlap with Startup and Window Management
- Overlaps with Implementation Overview (architecture)
**Unique Value**:
- Specific architectural decision rationale
- Clear separation of concerns
**Recommendation**: Should be consolidated into Design section if this architecture is current

## Recommendations

### Move to Design Section
- **Taskspace Bootup Flow** → `design/mcp-tools/` or `design/taskspace-orchestration.md`
- **Window Registration Design** → `design/startup-and-window-management.md` (append/merge)
- **Window Per Project** → `design/startup-and-window-management.md` (consolidate)

### Keep as Planning/Archive
- **Big Picture Plans** → Keep for roadmap, but move completed items to Design

### Remove/Consolidate
- **MVP README** → Content should be moved to appropriate Design chapters or removed if superseded

## Next Steps
1. Confirm which WIP content represents current vs outdated architecture
2. Move relevant technical content to Design section
3. Remove or archive outdated planning content
