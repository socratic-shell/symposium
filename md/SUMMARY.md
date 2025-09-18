A# Summary

<!-- 
    AGENTS: Please keep this design documentation up-to-date/

    Also, please review appropriate chapters and research reports
    whne looking to learn more details about a specific area.
-->

- [Introduction](./introduction.md)
- [Installation and setup](./setup.md)

# Design and implementation

- [Implementation Overview](./design/implementation-overview.md)
- [mdbook Conventions](./design/mdbook-conventions.md)
- [Persistent Agent Sessions](./design/persistent-agent-sessions.md)
- [Guidance and Initialization](./design/guidance-and-initialization.md)
- [Taskspace Deletion System](./design/taskspace-deletion.md)
- [IPC Communication and Daemon Architecture](./design/daemon.md)
    - [IPC message type reference](./design/ipc_message_type_reference.md)
- [Agent manager](./design/agent-manager.md)
- [Socratic Shell MCP server + IDE extension specifics]()
    - [MCP Server Tools](./design/mcp-server.md)
        - [IDE Integration Tools](./design/mcp-tools/ide-integration.md)
        - [Code Walkthrough Tools](./design/mcp-tools/walkthroughs.md)
        - [Synthetic Pull Request Tools](./design/mcp-tools/synthetic-prs.md)
        - [Taskspace Orchestration Tools](./design/mcp-tools/taskspace-orchestration.md)
        - [Reference System Tools](./design/mcp-tools/reference-system.md)
        - [Rust Development Tools](./design/mcp-tools/rust-development.md)
    - [Socratic Shell Reference System](./design/socratic-shell-ref-system.md)
    - [Ask Socratic Shell](./design/ask-socratic-shell.md)
    - [Code walkthroughs](./design/walkthroughs.md)
        - [Walkthrough format](./design/walkthrough-format.md)
        - [Comment Interactions](./design/walkthrough-comment-interactions.md)
    - [Dialect language](./design/dialect-language.md)
- [Symposium application specifics]()
    - [Startup and Window Management](./design/startup-and-window-management.md)
    - [Stacked Windows](./design/stacked-windows.md)
    - [Window Stacking Design](./design/window-stacking-design.md)
    - [Window Stacking Scenario Walkthrough](./design/window-stacking-scenario.md)

# Requests for Dialog

<!--

A "Request for Dialog" (RFD) is Socratic Shell's version of the RFC process.

Each entry here maps to a file whose name is the shorthand name for the RFD, e.g.,  `./rfds/ide-operations.md`. 

The RFD tracks the feature's progress from design to implementation. They are living documents that are kept up-to-date until the feature is completed.

RFDs may have other associated files in a directory, e.g., `./rfds/ide-operations/auxiliary-data.md`.

RFDs are moved from section to section by the Socratic Shell team members only.

People can propose an RFD by create a PR adding a new file into the early drafts section. It should have a suitable name using "kebab-case" conventions.

-->

- [About RFDs](./rfds/README.md)
    - [RFD Template](./rfds/TEMPLATE.md)
    - [Terminology and Conventions](./rfds/terminology-and-conventions.md)
- [Preview]() <!-- Close to ready, highlighted for attention -->
    - [Taskspace Deletion Dialog Confirmation](./rfds/taskspace-deletion-dialog-confirmation.md)
    - [Rust Crate Sources Tool](./rfds/rust-crate-sources-tool.md)
- [Draft]() <!-- Early drafts, people start things in this section -->
    - [Tile-based Window Management](./rfds/tile-based-window-management.md)
- [To be removed (yet?)]() <!-- Decided against doing this for now -->
- [Completed]() <!-- Work is complete -->
    - [Introduce RFD Process](./rfds/introduce-rfd-process.md)

# Work in Progress

- [Big picture plans](./work-in-progress/big-picture.md)
- [MVP](./work-in-progress/mvp/README.md)
    - [Taskspace Bootup Flow](./work-in-progress/mvp/taskspace-bootup-flow.md)
    - [Window Registration Design](./work-in-progress/mvp/window-registration-design.md)
    - [Window Per Project](./work-in-progress/mvp/window-per-project.md)

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
