# Taskspace Architecture

This document describes the core architectural decisions for Symposium taskspaces - the essential components and design patterns that would be expensive to rederive.

## Core Concept

A **taskspace** is an isolated work environment containing:
- **AI Agent**: Autonomous coding assistant (Claude Code, Q CLI, etc.)
- **Project Code**: The codebase being worked on  
- **Development Environment**: Tools, compilers, runtime dependencies
- **Communication Hub**: Daemon providing HTTP-based IPC with message buffering

## Container Architecture

Each taskspace runs as a **pod** with multiple containers working together:

### Agent Container
- **Purpose**: Runs the AI agent in a persistent tmux session
- **Contents**: Agent binary (Claude Code/Q CLI), tmux session manager, conversation history
- **Access**: SSH connection drops directly into agent conversation
- **Session Management**: tmux configured for good scrollback, agent auto-restarts on crash

### Development Environment Container  
- **Purpose**: Provides project-specific toolchain and IDE connectivity
- **Contents**: Language toolchains (detected from project), development tools, SSH server
- **Access**: VSCode Remote-SSH, IntelliJ remote development, terminal access
- **Integration**: Shares project files with agent container via mounted volumes

## Communication Model

### HTTP-Based Daemon
- **Migration from Unix Sockets**: HTTP communication enables container-to-container coordination
- **Message Buffering**: Daemon buffers messages when clients disconnect/reconnect
- **Event Replay**: Reconnecting clients receive all missed messages in order
- **Multi-Client Support**: Coordinates between agent, IDE connections, and Symposium app

### Message Flow
- Agent progress updates → Daemon buffer → Symposium app UI
- User commands from app → Daemon → Agent container
- IDE connections maintain independent communication with dev container

## Taskspace Layout

Each taskspace gets a host directory with shared mounts across containers:

### Host Directory Structure
```
/path/to/project.symposium
├── common/               # Shared resources across all containers
│   ├── ssh/
│   │   ├── agent_key     # Optional: SSH key for agent git operations
│   │   └── agent_key.pub
│   ├── bin/
│   │   └── socratic-shell-mcp # MCP server binary
│   └── config/           # Shared configuration files
└── taskspace-{uuid}/     # Per taskspace data
  ├── .taskspace.json     # Logs, metadata, progress records
  ├── .agent/             # Agent-specific dotfiles and configuration  
  └── {project-name}/     # Git checkout (cannot start with '.')
```

### Container Mount Strategy

**Agent Container Mounts:**
```
~/.claude/          ← /path/to/project.symposium/task-{uuid}/.agent
~/{project-name}    ← /path/to/project.symposium/task-{uuid}/{project-name}
~/.ssh/             ← /path/to/project.symposium/common/ssh
~/bin/              ← /path/to/project.symposium/common/bin
```

**Dev Container Mounts:**
```
~/{project-name}    ← /path/to/project.symposium/task-{uuid}/{project-name}
~/.ssh/             ← /path/to/project.symposium/common/ssh
~/bin/              ← /path/to/project.symposium/common/bin
```

## SSH Access Model

### Multi-Level Access
- **User SSH**: Personal keys for IDE connectivity, SSH agent forwarding for personal git operations
- **Agent SSH**: Optional shared key pair for agent git operations (same keys across all agents)
- **Security Boundary**: Agents never access user's personal SSH keys

### IDE Integration Patterns
- **VSCode Remote-SSH**: Connects to development container for full IDE experience
- **Terminal Direct**: SSH to agent container for direct AI conversation
- **Multi-Editor Support**: IntelliJ, RubyMine via SSH remote development

## tmux Session Management

### Agent Persistence
- Agent runs in named tmux session with extensive scrollback history
- SSH connections attach directly to agent conversation (no shell prompt)
- Session survives agent crashes and container restarts
- Optimized for long-form conversations with good scroll/copy support

### Session Recovery
- If agent exits, tmux session remains; agent auto-restarts in background
- SSH disconnection doesn't terminate agent - conversation continues
- New SSH connections attach to existing session

## Project Creation Workflow

### Setup Wizard
Each `.symposium` directory is created through a configuration wizard that handles deployment setup:

**User Configuration:**
1. **Location & Host**: User specifies path and target host
   - Local: `~/projects/my-app.symposium`
   - Remote: `ssh://dev-server/home/user/my-app.symposium`
2. **SSH Access**: Optional checkbox for agent SSH key generation
3. **Agent Type**: Choose default agent (Claude Code, Q CLI, etc.)

**Infrastructure Provisioning:**
- **Directory Creation**: Symposium creates `.symposium` structure at target location
- **SSH Key Setup**: Generates agent SSH keys if enabled, installs at target location
- **Binary Distribution**: Downloads/builds `socratic-shell-mcp` binary for target platform
- **Configuration**: Writes `project.json` with agent settings and deployment info

**Result:**
```
{user-specified-path}/
├── common/
│   ├── ssh/                 # Created if SSH access enabled
│   │   ├── agent_key
│   │   └── agent_key.pub
│   ├── bin/
│   │   └── socratic-shell-mcp    # Platform-appropriate binary
│   └── config/
│       └── project.json     # Agent type, SSH settings, host info
└── (taskspace directories created as needed)
```

### Deployment Flexibility
- **Location Independence**: Same structure works locally or on any SSH-accessible host
- **Team Collaboration**: Multiple developers can share a remote `.symposium` directory
- **One-Time Setup**: Infrastructure concerns handled at project creation, not per-taskspace

## Deployment Evolution

### Current Focus: Local Development
- **Phase 1**: localhost/macOS experiments and validation
- Project creation wizard supports local `.symposium` directories
- SSH connections to localhost ports for IDE integration

### Future: Remote Deployment  
- **Same Architecture**: SSH to remote Mac/Linux machines via project creation wizard
- **Extended Targets**: Cloud deployment, Kubernetes orchestration, GitHub Codespaces
- Multi-tenant support with isolated taskspaces per user

## Key Design Decisions

### Why Containers?
- **Agent Isolation**: Crash recovery, resource limits, clean shutdown
- **Reproducible Environments**: Consistent toolchains across deployment targets
- **Multi-Editor Support**: SSH-based access works with any IDE

### Why HTTP vs Unix Sockets?
- **Container Boundaries**: HTTP crosses container network boundaries
- **Message Buffering**: Enables reliable communication across disconnects
- **Remote Ready**: Same communication model works locally and remotely

### Why tmux for Sessions?
- **Persistence**: Conversation history survives connection drops
- **Scrollback**: Essential for long AI conversations and code review
- **Familiar**: Standard tool that integrates well with terminal workflows

### Why Shared Common Directory?
- **Binary Distribution**: Same MCP server binary across all taskspaces
- **Key Management**: Centralized SSH key distribution
- **Configuration**: Shared tools and settings without duplication

This architecture provides a foundation for AI-assisted development that scales from individual experimentation to team collaboration while maintaining familiar development workflows.