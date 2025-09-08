# Experiment 2: Containerized Agent

**Status**: Planning  
**Started**: 2025-01-08  
**Objective**: Validate that we can run an AI agent inside a container with SSH access and effective file system integration.

## Core Hypothesis

We can create a container that:
1. Runs an AI agent (Claude Code or Q CLI) in a tmux session
2. Accepts SSH connections that drop directly into agent interaction
3. Provides the agent with access to project files through volume mounts
4. Handles secrets (SSH keys, auth tokens) through simple volume mounts
5. Auto-restarts the agent if it exits, maintaining availability

## Success Criteria

### Minimum Viable Success
- [ ] Container runs with Claude Code or Q CLI inside tmux
- [ ] SSH connection drops directly into agent conversation
- [ ] Agent can read and write files in mounted project directory  
- [ ] Agent exits cleanly terminate SSH session

### Stretch Goals
- [ ] Agent auto-restarts after exit, new tmux session ready
- [ ] Secrets mounted from host `~/.ssh` directory work
- [ ] Agent can execute git commands on project files
- [ ] Container startup time is reasonable (< 10 seconds)

## Technical Approach

### Container Architecture
```
Container:
├── SSH Server (openssh-server)
├── tmux session manager
├── Agent (Claude Code/Q CLI)
├── Mounted volumes:
│   ├── /workspace -> project directory
│   ├── /secrets -> ~/.ssh (read-only)
│   └── /config -> agent configuration
└── Entry script handling SSH → tmux → agent flow
```

### Key Design Decisions

**SSH as Entry Point**: SSH connection immediately drops into tmux session with running agent, no shell prompt access.

**Volume Mounts for Everything**: Project files, secrets, and configuration all handled through simple volume mounts rather than complex injection mechanisms.

**tmux Session Management**: Container maintains a persistent tmux session; SSH connects to existing session or creates new one if agent restarted.

**Agent Selection**: Container configured at build/runtime for specific agent (Claude Code vs Q CLI).

## Implementation Plan

### Phase 1: Basic Container
1. Create Dockerfile with SSH server + tmux + Claude Code
2. Configure SSH to execute custom entry script instead of shell
3. Entry script: ensure tmux session exists with agent running, attach to it
4. Test: SSH connection → immediate agent interaction

### Phase 2: File System Integration  
1. Add volume mount for project directory
2. Configure agent to use mounted directory as workspace
3. Test: agent can list, read, edit files; changes persist to host

### Phase 3: Secret Management
1. Add volume mount for ~/.ssh directory (read-only)
2. Configure agent to use mounted SSH keys
3. Test: agent can clone repositories, push changes

### Phase 4: Robustness
1. Implement agent restart logic (if agent exits, restart in background)
2. Handle SSH disconnection/reconnection gracefully  
3. Add container health checks
4. Test: agent crashes → auto-restart → new SSH connection works

## Open Questions

1. **Agent Configuration**: How do we pass initial context/instructions to the containerized agent?

2. **Resource Limits**: What CPU/memory limits should we set? How do we handle resource exhaustion?

3. **Networking**: Do agents need outbound internet access? How do we handle firewall/proxy scenarios?

4. **Data Persistence**: Should agent conversation history persist across container restarts?

5. **Multi-User**: Can multiple users SSH into the same container simultaneously, or do we need one container per user?

## Risk Mitigation

**Risk**: SSH security vulnerabilities  
**Mitigation**: Run SSH on non-standard port, use key-based auth only, consider user namespaces

**Risk**: Agent crashes/hangs breaking container  
**Mitigation**: Process monitoring, health checks, automatic restart logic  

**Risk**: File system permission issues  
**Mitigation**: Use matching UIDs between host and container, test with different project directory permissions

**Risk**: Poor performance due to containerization overhead  
**Mitigation**: Benchmark container vs native agent performance, optimize if needed

## Next Steps

1. Create basic Dockerfile and test SSH connectivity
2. Add agent integration and test basic interaction
3. Add file system mounts and test file operations  
4. Iterate on robustness and user experience

## Related Documentation

- [Agent Container Architecture](../architecture/agent-container.md) - Future vision for agent containers
- [Container Orchestration](../architecture/orchestration.md) - How multiple containers will be managed
- [Implementation Overview](../../design/implementation-overview.md) - Current MVP implementation