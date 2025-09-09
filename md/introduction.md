## Symposium

Symposium is a "meta-IDE" that orchestrates multiple *taskspaces*. A *taskspace* is the combination of an AI agent and a window (could be an IDE, a terminal, an Emacs.app window, etc) that permits editing. Symposium is intentionally generic with respect to what AI agents are in use and what kind of windows are in use.

Key capabilities include:

- **Taskspace Management**: Create, organize, and switch between multiple AI-powered workspaces
- **Window Orchestration**: Coordinate window positioning and focus across different applications
- **Persistent Agents**: Run AI agents in background sessions that survive disconnections
- **Cross-Application Integration**: Bridge AI agents with IDEs, terminals, and other development tools

Symposium includes an MCP server that provides tools for AI agents to spawn new taskspaces, report progress, and coordinate with the broader development environment. The system supports both synchronous agents (running in terminal foreground) and persistent agents (running in background tmux sessions) to accommodate different workflow needs.