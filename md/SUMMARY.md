# Summary

<!-- 
    AGENTS: Please keep this design documentation up-to-date/

    Also, please review appropriate chapters and research reports
    whne looking to learn more details about a specific area.
-->

- [Introduction](./introduction.md)
- [Interface plan](./interface-plan.md)
- [Implementation Overview](./implementation-overview.md)

# Design

- [Symposium Reference System](./design/symposium-ref-system.md)
- [Ask Socratic Shell](./design/ask-socratic-shell.md)
- [IPC Communication and Daemon Architecture](./design/daemon.md)
- [Window Stacking Design](./window-stacking-design.md)
- [Window Stacking Scenario Walkthrough](./window-stacking-scenario.md)

# Research reports

- [Taskspace Implementation Guide](./research/taskspace-implementation-guide.md)
- [macOS Sequoia 15.6 Accessibility Permission Research Report](./research/macos_accessibility_research_report.md)
- [AeroSpace's Approach to Window Following](./research/aerospace-approach-to-window-following.md)
- [Reliable identification of windows (a surprisingly hard topic)]()
    - [Developer-focused window identification](./research/developer_focused_window_identification.md)
    - [Mac OS Window identification research by Claude](./research/macos_window_identification_research_claude.md)
    - [Mac OS Window identification research by Gemini](./research/macos_window_identification_research_gemini.md)
- [Why We Rejected Core Graphics Services APIs](./research/why-we-rejected-cgs-apis.md)
    - [Core Graphics Window APIs](./research/cg-window-apis.md)
    - [How macOS Applications Respond to CG APIs](./research/how-mac-os-applications-respond-to-cg-apis.md)
    - [CGS API Security Restrictions Research Report](./research/cgs-api-security-restrictions.md)

# Dialectic Integration Documentation

<!-- Research and design docs from dialectic integration -->

- [User Guide]() <!-- From dialectic -->
    - [Installation](./installation.md) 
    - [Quick start](./quick-start.md)
    - [Features]()
        - [Code walkthroughs and Ask Socratic Shell](./walkthroughs.md)
        - [Synthetic Pull Requests](./synthetic-pr.md)
        - [IDE Capabilities](./ide-capabilities.md)
        - [Review format](./review-format.md)
    - [Frequently asked questions](./faq.md)

- [Development and contribution guide]()
    - [Building and testing](./design/build-and-test.md)
    - [System overview](./design/overview.md)
        - [Communication protocol](./design/protocol.md)
        - [Security considerations](./design/security.md)
        - [AI Guidance design considerations](./design/ai-guidance.md)
        - [Codebase structure](./design/codebase-structure.md)
    - [How each feature works]()
        - [Code walkthroughs](./design/code-walkthroughs.md)
        - [Synthetic Pull Requests](./design/synthetic-pr.md)
        - [Ask Socratic Shell](./design/ask-socratic-shell.md)
        - [IDE Capabilities](./design/ide-capabilities.md)
    - [MCP server](./design/mcp-server.md)
        - [MCP Tool interface](./design/mcp-tool-interface.md)
    - [VSCode extension](./design/extension.md)
    - [Walkthrough format](./design/walkthrough-format.md)
    - [Dialect language](./design/dialect-language.md)

- [Research reports from dialectic]()
    - [Markdown to HTML in VSCode Extensions](./references/markdown-to-html-in-vscode.md)
    - [VSCode Extension Communication Patterns](./references/cli-extension-communication-guide.md)
    - [VSCode Sidebar Panel Research](./references/vscode-extensions-sidebar-panel-research-report.md)
    - [Language Server Protocol Overview](./references/lsp-overview/README.md)
        - [Base Protocol](./references/lsp-overview/base-protocol.md)
        - [Language Features](./references/lsp-overview/language-features.md)
        - [Implementation Guide](./references/lsp-overview/implementation-guide.md)
        - [Message Reference](./references/lsp-overview/message-reference.md)
    - [Unix IPC Message Bus Implementation Guide](./references/unix-message-bus-architecture.md)
    - [VSCode PR Extension Research](./references/vscode-extensions-dev-pattern.md)
    - [VSCode File System Watching APIs](./references/VS-Code-file-system-watching.md)
    - [Synthetic PRs in VSCode](./references/Synthetic-PRs-in-vscode.md)
    - [VSCode Git Extension API Capabilities](./references/VSCode-Git-Extension-API-capabilities.md)
    - [Comment System Architecture for PR Reviews](./references/comment-system-on-pr.md)
    - [Diff Visualization Strategies](./references/diff-visualization.md)
    - [Cumulative Diff Visualization Analysis](./references/diff-visualization-cumulative.md)
    - [Copilot Integration Guide](./references/copilot-guide.md)
    - [Copilot Integration Guide 2](./references/copilot-guide-2.md)
    - [Copilot Integration Guide 3](./references/copilot-guide-3.md)
    - [VSCode Comments API Reply Button Implementation](./references/VSCode-Comments-API-Reply-Button.md)
    - [VSCode WebviewView State Persistence](./references/vscode-webview-state-persistence.md)