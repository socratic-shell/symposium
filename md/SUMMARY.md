# Summary

<!-- 
    AGENTS: Please keep this design documentation up-to-date/

    Also, please review appropriate chapters and research reports
    whne looking to learn more details about a specific area.
-->

- [Introduction](./introduction.md)
- [Installation and setup](./setup.md)

<!--
    AGENTS: "Research Reports" are in-depth documents you can read to learn more
    about a particular topic
-->

# Design and implementation

- [Implementation Overview](./design/implementation-overview.md)
- [Startup and Window Management](./design/startup-and-window-management.md)
- [Stacked Windows](./design/stacked-windows.md)
- [Persistent Agent Sessions](./design/persistent-agent-sessions.md)
- [MCP Server Tools](./design/mcp-server.md)
    - [IDE Integration Tools](./design/mcp-tools/ide-integration.md)
    - [Code Walkthrough Tools](./design/mcp-tools/walkthroughs.md)
    - [Synthetic Pull Request Tools](./design/mcp-tools/synthetic-prs.md)
    - [Taskspace Orchestration Tools](./design/mcp-tools/taskspace-orchestration.md)
    - [Reference System Tools](./design/mcp-tools/reference-system.md)
- [Symposium Reference System](./design/symposium-ref-system.md)
- [Ask Socratic Shell](./design/ask-socratic-shell.md)
- [IPC Communication and Daemon Architecture](./design/daemon.md)
    - [IPC message type reference](./design/ipc_message_type_reference.md)
- [Agent manager](./design/agent-manager.md)
- [Code walkthroughs](./design/walkthroughs.md)
    - [Walkthrough format](./design/walkthrough-format.md)
    - [Comment Interactions](./design/walkthrough-comment-interactions.md)
- [Window Stacking Design](./design/window-stacking-design.md)
- [Window Stacking Scenario Walkthrough](./design/window-stacking-scenario.md)
- [Dialect language](./design/dialect-language.md)

# Work in Progress

- [Startup Window Management Implementation](./work-in-progress/startup-window-management.md)
- [Big picture plans](./work-in-progress/big-picture.md)
    - [Architecture](./work-in-progress/big-picture/architecture.md)
    - [Experiments](./work-in-progress/big-picture/experiments.md)
        - [Experiment 1: HTTP + Buffering Daemon](./work-in-progress/big-picture/experiments/experiment-1-http-buffering-daemon.md)
        - [Experiment 2: Containerized Agent](./work-in-progress/big-picture/experiments/experiment-2-containerized-agent.md)
    - [Steps](./work-in-progress/big-picture/steps.md)
- [MVP](./work-in-progress/mvp/README.md)
    - [Implementation Plan](./work-in-progress/mvp/implementation-plan.md)
    - [Taskspace Bootup Flow](./work-in-progress/mvp/taskspace-bootup-flow.md)
    - [Window Registration Design](./work-in-progress/mvp/window-registration-design.md)
    - [Window Per Project](./work-in-progress/mvp/window-per-project.md)
- [Project Window Visual Design](./work-in-progress/project-window-visual-design.md)
- [Centered Panel Implementation Plan](./work-in-progress/centered-panel-implementation-plan.md)
- [Git Worktrees Migration Plan](./work-in-progress/git-worktrees-migration.md)
- [Older material]()
    - [Interface plan](./work-in-progress/mvp/interface-plan.md)
    - [Symposium Panel Prototype](./work-in-progress/mvp/symposium-panel-prototype.md)
    - [Dock-Activated Interface](./work-in-progress/dock-activated-interface.md)

<!--
    AGENTS: "Research Reports" are in-depth documents you can read to learn more
    about a particular topic
-->

# Research reports

- [Taskspace Implementation Guide](./research/taskspace-implementation-guide.md)
- [macOS Sequoia 15.6 Accessibility Permission Research Report](./research/macos_accessibility_research_report.md)
- [AeroSpace's Approach to Window Following](./research/aerospace-approach-to-window-following.md)
- [Reliable identification of windows (a surprisingly hard topic)]()
    - [Developer-focused window identification](./research/developer_focused_window_identification.md)
    - [Mac OS Window identification research by Claude](./research/macos_window_identification_research_claude.md)
    - [Mac OS Window identification research by Gemini](./research/macos_window_identification_research_gemini.md)
    - [VSCode Window Title Control for macOS Integration](./research/vscode-window-title-control-report.md)
- [Why We Rejected Core Graphics Services APIs](./research/why-we-rejected-cgs-apis.md)
    - [Core Graphics Window APIs](./research/cg-window-apis.md)
    - [How macOS Applications Respond to CG APIs](./research/how-mac-os-applications-respond-to-cg-apis.md)
    - [CGS API Security Restrictions Research Report](./research/cgs-api-security-restrictions.md)
- [Markdown to HTML in VSCode Extensions](./research/markdown-to-html-in-vscode.md)
- [VSCode Extension Communication Patterns](./research/cli-extension-communication-guide.md)
- [VSCode Sidebar Panel Research](./research/vscode-extensions-sidebar-panel-research-report.md)
- [Language Server Protocol Overview](./research/lsp-overview/README.md)
    - [Base Protocol](./research/lsp-overview/base-protocol.md)
    - [Language Features](./research/lsp-overview/language-features.md)
    - [Implementation Guide](./research/lsp-overview/implementation-guide.md)
    - [Message Reference](./research/lsp-overview/message-reference.md)
- [Unix IPC Message Bus Implementation Guide](./research/unix-message-bus-architecture.md)
- [VSCode PR Extension Research](./research/vscode-extensions-dev-pattern.md)
- [VSCode File System Watching APIs](./research/VS-Code-file-system-watching.md)
- [Synthetic PRs in VSCode](./research/Synthetic-PRs-in-vscode.md)
- [VSCode Git Extension API Capabilities](./research/VSCode-Git-Extension-API-capabilities.md)
- [Comment System Architecture for PR Reviews](./research/comment-system-on-pr.md)
- [Diff Visualization Strategies](./research/diff-visualization.md)
- [Cumulative Diff Visualization Analysis](./research/diff-visualization-cumulative.md)
- [Copilot Integration Guide](./research/copilot-guide.md)
- [Copilot Integration Guide 2](./research/copilot-guide-2.md)
- [Copilot Integration Guide 3](./research/copilot-guide-3.md)
- [VSCode Comments API Reply Button Implementation](./research/VSCode-Comments-API-Reply-Button.md)
- [VSCode WebviewView State Persistence](./research/vscode-webview-state-persistence.md)
