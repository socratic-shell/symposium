# Summary

<!-- 
    AGENTS: Please keep this design documentation up-to-date/

    Also, please review appropriate chapters and research reports
    whne looking to learn more details about a specific area.
-->

- [Introduction](./introduction.md)
- [About](./about.md)
- [Get started](./get-started/index.md)
    - [Install](./get-started/install.md)
    - [Create Symposium project](./get-started/symposium-project.md)
    - [Taskspaces](./get-started/taskspaces.md)
    - [Walkthroughs](./get-started/walkthroughs.md)
    - [Get Rust crate sources](./get-started/rust_crate_source.md)
    - [Say "hi"](./get-started/say-hi.md)
    - [Unopinionated setup](./get-started/unopinionated.md)
- [Tenets](./tenets.md)
- [Contribute](./contribute.md)

# User's reference

- [Taskspaces](./ref/taskspaces.md)
- [Interactive walkthroughs](./ref/walkthroughs.md)
- [IDE integration](./ref/ide-integration.md)
- [Collaborative prompts](./ref/collaborative-prompts.md)
- [Symposium references](./ref/symposium-ref.md)

# Requests for Dialog

<!--

A "Request for Dialog" (RFD) is Symposium's version of the RFC process.

Each entry here maps to a file whose name is the shorthand name for the RFD, e.g.,  `./rfds/ide-operations.md`. 

The RFD tracks the feature's progress from design to implementation. They are living documents that are kept up-to-date until the feature is completed.

RFDs may have other associated files in a directory, e.g., `./rfds/ide-operations/auxiliary-data.md`.

RFDs are moved from section to section by the Symposium team members only.

People can propose an RFD by create a PR adding a new file into the early drafts section. It should have a suitable name using "kebab-case" conventions.

-->

- [About RFDs](./rfds/index.md)
    - [RFD Template](./rfds/TEMPLATE.md)
    - [Terminology and Conventions](./rfds/terminology-and-conventions.md)
- [Preview](./rfds/preview.md) <!-- Close to ready, highlighted for attention -->
    - [Taskspace Deletion Dialog Confirmation](./rfds/taskspace-deletion-dialog-confirmation.md)
    - [Rust Crate Sources Tool](./rfds/rust-crate-sources-tool.md)
- [Invited -- help wanted!](./rfds/invited.md) <!-- This where I want someone to take it over -->
- [Draft](./rfds/draft.md) <!-- Early drafts, people start things in this section -->
    - [Persistent Agents](./rfds/persistent-agents.md)
    - [Tile-based Window Management](./rfds/tile-based-window-management.md)
    - [GitDiff Elements in Walkthroughs](./rfds/gitdiff-elements.md)
- [To be removed (yet?)](./rfds/to-be-removed.md) <!-- Decided against doing this for now -->
- [Completed](./rfds/completed.md) <!-- Work is complete -->
    - [Introduce RFD Process](./rfds/introduce-rfd-process.md)
    - [IPC Actor Refactoring](./rfds/ipc-actor-refactoring.md)

# Design and implementation

- [Implementation Overview](./design/implementation-overview.md)
- [mdbook Conventions](./design/mdbook-conventions.md)
- [Collaborative prompt engineering](./collaborative-prompting.md)
- [IPC Communication and Daemon Architecture](./design/daemon.md)
    - [IPC message type reference](./design/ipc_message_type_reference.md)
- [Symposium MCP server + IDE extension specifics](./design/mcp-server-ide.md)
    - [MCP Server Actor Architecture](./design/mcp-server-actor-architecture.md)
    - [Guidance and Initialization](./design/guidance-and-initialization.md)
    - [MCP Server Tools](./design/mcp-server.md)
        - [IDE Integration Tools](./design/mcp-tools/ide-integration.md)
        - [Code Walkthrough Tools](./design/mcp-tools/walkthroughs.md)
        - [Synthetic Pull Request Tools](./design/mcp-tools/synthetic-prs.md)
        - [Taskspace Orchestration Tools](./design/mcp-tools/taskspace-orchestration.md)
        - [Reference System Tools](./design/mcp-tools/reference-system.md)
        - [Rust Development Tools](./design/mcp-tools/rust-development.md)
    - [Symposium Reference System](./design/symposium-ref-system.md)
    - [Discuss in Symposium](./design/discuss-in-symposium.md)
    - [Code walkthroughs](./design/walkthroughs.md)
        - [Walkthrough format](./design/walkthrough-format.md)
        - [Comment Interactions](./design/walkthrough-comment-interactions.md)
    - [Dialect language](./design/dialect-language.md)
- [Symposium application specifics](./design/symposium-app-specifics.md)
    - [Startup and Window Management](./design/startup-and-window-management.md)
    - [Stacked Windows](./design/stacked-windows.md)
    - [Window Stacking Design](./design/window-stacking-design.md)
    - [Window Stacking Scenario Walkthrough](./design/window-stacking-scenario.md)
    - [Taskspace Deletion System](./design/taskspace-deletion.md)
- [Persistent Agent Sessions](./design/persistent-agent-sessions.md)
    - [Agent manager](./design/agent-manager.md)

<!--
    AGENTS: "Research Reports" are in-depth documents you can read to learn more
    about a particular topic
-->

# Research reports

- [Language Server Protocol Overview](./research/lsp-overview/README.md)
    - [Base Protocol](./research/lsp-overview/base-protocol.md)
    - [Language Features](./research/lsp-overview/language-features.md)
    - [Implementation Guide](./research/lsp-overview/implementation-guide.md)
    - [Message Reference](./research/lsp-overview/message-reference.md)
