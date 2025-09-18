# Chapters to Remove from Mdbook

Here are the current "WIP" chapters from `md/SUMMARY.md`.

I'm going to mark them with either:

* X -- outdated, delete
* ? -- preserve for now, we will revisit

- X [Repository Restructure: Socratic Shell + Symposium Split](./work-in-progress/rebrand.md)
- X [Startup Window Management Implementation](./work-in-progress/startup-window-management.md)
- X [Triple-Tickification: XML to Markdown Code Blocks](./work-in-progress/triple-tickification.md)
- ? [Big picture plans](./work-in-progress/big-picture.md)
    - X [Architecture](./work-in-progress/big-picture/architecture.md)
    - X [Experiments](./work-in-progress/big-picture/experiments.md)
        - X [Experiment 1: HTTP + Buffering Daemon](./work-in-progress/big-picture/experiments/experiment-1-http-buffering-daemon.md)
        - X [Experiment 2: Containerized Agent](./work-in-progress/big-picture/experiments/experiment-2-containerized-agent.md)
    - X [Steps](./work-in-progress/big-picture/steps.md)
- ? [MVP](./work-in-progress/mvp/README.md)
    - X [Implementation Plan](./work-in-progress/mvp/implementation-plan.md)
    - ? [Taskspace Bootup Flow](./work-in-progress/mvp/taskspace-bootup-flow.md)
    - ? [Window Registration Design](./work-in-progress/mvp/window-registration-design.md)
    - ? [Window Per Project](./work-in-progress/mvp/window-per-project.md)
- X [Project Window Visual Design](./work-in-progress/project-window-visual-design.md)
- X [Centered Panel Implementation Plan](./work-in-progress/centered-panel-implementation-plan.md)
- X [Git Worktrees Migration Plan](./work-in-progress/git-worktrees-migration.md)
- X [Older material]()
    - X [Interface plan](./work-in-progress/mvp/interface-plan.md)
    - X [Symposium Panel Prototype](./work-in-progress/mvp/symposium-panel-prototype.md)
    - X [Dock-Activated Interface](./work-in-progress/dock-activated-interface.md)


## Work in Progress - Candidates for Removal

### Outdated planning that can be deleted, either because completed or no longer relevant
- `./work-in-progress/startup-window-management.md` - Only contains title, appears empty/abandoned
- `./work-in-progress/triple-tickification.md` - XML to markdown conversion appears completed
- `./work-in-progress/rebrand.md` - Repository restructure appears completed
- `./work-in-progress/mvp/interface-plan.md`
- `./work-in-progress/mvp/symposium-panel-prototype.md`
- `./work-in-progress/dock-activated-interface.md`

## Research Reports - Candidates for Removal

### Duplicate/Redundant Research
- `./research/copilot-guide-2.md` - Duplicate of copilot research
- `./research/copilot-guide-3.md` - Third iteration, likely supersedes earlier ones
- `./research/macos_window_identification_research_gemini.md` - Duplicate research by different AI
- `./research/macos_window_identification_research_claude.md` - Duplicate research by different AI

### Rejected/Obsolete Approaches
- `./research/SwiftUI-openWindow-debugging.md` - Extensive debugging of potentially abandoned approach
- `./research/why-we-rejected-cgs-apis.md` - Documents rejected approach
- `./research/cg-window-apis.md` - Part of rejected CGS approach
- `./research/how-mac-os-applications-respond-to-cg-apis.md` - Part of rejected CGS approach
- `./research/cgs-api-security-restrictions.md` - Part of rejected CGS approach

### Potentially Outdated Research
- `./research/vscode-pr-extension-research.md` - May be superseded by current implementation
- `./research/diff-visualization-cumulative.md` - May be superseded by current approach

### Empty/Minimal Files
- `./research/api.md` - Only 16 bytes, likely empty
- `./research/configuration.md` - Only 26 bytes, likely empty
