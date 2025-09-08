# Agent Container Architecture

The agent container is the core execution environment for AI coding assistants, providing persistent sessions, crash recovery, and seamless connectivity for both human and programmatic interaction.

## Overview

The agent container hosts:
- **AI Agent**: Claude Code, Q CLI, or other compatible agent
- **Session Management**: Tmux-based persistence and recovery
- **IPC Services**: symposium-mcp binary providing MCP server, daemon, and client functionality
- **SSH Access**: Direct connection to agent conversation
- **Supervision**: Automatic restart and crash recovery

## Agent Session Management

### Tmux Configuration

Agent runs in a named tmux session with optimized settings for long-running conversations:

```bash
# /agent/tmux.conf - Agent-specific configuration
set -g history-limit 100000              # Extensive conversation history
set -g mouse on                          # Mouse scrolling support  
set -g terminal-overrides 'xterm*:smcup@:rmcup@'  # Better scrollback
set -g base-index 1                      # Start numbering at 1
setw -g mode-keys vi                     # Vi-style copy mode

# Disable session/window management to prevent user confusion
unbind c        # No new windows
unbind &        # No kill window  
unbind s        # No session browser
set -g status off   # Clean interface for agent interaction
```

### Session Initialization

Agent session is created with proper working directory and environment:

```bash
#!/bin/bash
# /usr/local/bin/init-agent-session

# Create persistent tmux session
tmux new-session -d -s agent-session -c /workspace \
  -e SYMPOSIUM_CONFIG=/agent/config.yaml \
  -e SYMPOSIUM_AUTH=/agent/auth \
  -e SYMPOSIUM_CONTEXT=/agent/context \
  '/usr/local/bin/agent-supervisor'

# Wait for session to be ready
sleep 1

echo "Agent session initialized: agent-session"
```

### Crash Recovery and Supervision

Agent supervisor handles automatic restarts while preserving conversation context:

```bash
#!/bin/bash
# /usr/local/bin/agent-supervisor

LOG_FILE="/agent/logs/supervisor.log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log "Agent supervisor starting"

consecutive_failures=0
max_failures=5

while true; do
    log "Starting agent (attempt: $((consecutive_failures + 1)))"
    
    # Start agent with conversation context preservation
    claude -c 2>&1 | tee -a /agent/logs/agent.log
    exit_code=$?
    
    # Clean exit - agent finished normally (user said goodbye)
    if [ $exit_code -eq 0 ]; then
        log "Agent exited cleanly"
        break
    fi
    
    # Crash handling
    consecutive_failures=$((consecutive_failures + 1))
    log "Agent crashed (exit code: $exit_code, failures: $consecutive_failures)"
    
    # Too many failures - give up
    if [ $consecutive_failures -ge $max_failures ]; then
        log "Too many consecutive failures, stopping supervisor"
        break
    fi
    
    # Exponential backoff
    delay=$((2 ** consecutive_failures))
    log "Restarting in $delay seconds..."
    sleep $delay
done

log "Agent supervisor exiting"
```

### Session Recovery

When container restarts, agent session is automatically restored:

```bash
#!/bin/bash
# Container entrypoint - /usr/local/bin/container-init

# Check for existing session
if tmux has-session -t agent-session 2>/dev/null; then
    echo "Resuming existing agent session"
    tmux attach-session -t agent-session
else
    echo "Creating new agent session"
    /usr/local/bin/init-agent-session
fi
```

## SSH Access Configuration

### Direct Agent Connection

SSH is configured to immediately connect users to the agent conversation:

```bash
# /etc/ssh/sshd_config additions
Port 2223
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes

# Force all connections directly to agent session
ForceCommand /usr/local/bin/tmux-attach

# User account configuration
Match User agent
    ForceCommand tmux attach-session -t agent-session
```

### Connection Script

```bash
#!/bin/bash
# /usr/local/bin/tmux-attach

# Ensure agent session exists
if ! tmux has-session -t agent-session 2>/dev/null; then
    /usr/local/bin/init-agent-session
fi

# Attach to agent session
exec tmux attach-session -t agent-session
```

### User Setup

```dockerfile
# In agent container Dockerfile
RUN useradd -m -s /usr/local/bin/tmux-attach agent && \
    mkdir -p /home/agent/.ssh && \
    chown agent:agent /home/agent/.ssh && \
    chmod 700 /home/agent/.ssh
```

## Symposium MCP Binary Integration

### Multi-Role Binary

The agent container runs a single `symposium-mcp` binary that serves multiple functions:

**As MCP Server:**
- Provides MCP tools to local agent (file operations, IDE integration, progress logging)
- Loads context from `/agent/context/` directory
- Uses auth from `/agent/auth/` directory

**As IPC Daemon:**
- Buffers messages with `buffer_` prefix for Symposium app replay
- Serves on localhost:8080 for pod-internal communication
- Handles connection management for other pod containers

**As IPC Client:**
- Connects to parent Symposium daemon when available
- Forwards buffered messages to host-level coordination

### Service Configuration

```yaml
# /agent/config.yaml
agent:
  type: "claude-code"  # or "q-cli"
  args: ["-c"]         # Resume with context
  
mcp_server:
  port: 8080
  context_dir: "/agent/context"
  auth_dir: "/agent/auth"
  
ipc_daemon:
  socket: "/tmp/symposium-daemon.sock"
  buffer_prefix: "buffer_"
  
logging:
  level: "info"
  file: "/agent/logs/symposium-mcp.log"
```

### Binary Startup

```bash
#!/bin/bash
# /usr/local/bin/start-symposium-mcp

# Ensure log directory exists
mkdir -p /agent/logs

# Start symposium-mcp in daemon mode
/usr/local/bin/symposium-mcp daemon \
  --config /agent/config.yaml \
  --port 8080 \
  --socket /tmp/symposium-daemon.sock \
  >> /agent/logs/symposium-mcp.log 2>&1 &

echo $! > /agent/symposium-mcp.pid
echo "symposium-mcp daemon started (PID: $!)"
```

## Programmatic Control

### Agent Interaction via Tmux

The Symposium app can send commands to the agent programmatically:

```bash
# Send user message to agent
tmux send-keys -t agent-session "Please analyze the authentication flow" Enter

# Interrupt agent if needed
tmux send-keys -t agent-session C-c

# Inject suggested commands
tmux send-keys -t agent-session "git commit -m 'implement OAuth flow'"

# Capture current agent output
tmux capture-pane -t agent-session -p
```

### Agent State Monitoring

```bash
# Check if agent is responsive
tmux list-sessions | grep agent-session

# Get current agent output
tmux capture-pane -t agent-session -S -100 -p

# Monitor agent activity
tail -f /agent/logs/agent.log
```

## Storage and Persistence

### Directory Structure

```
/agent/
├── config.yaml           # Agent and MCP configuration
├── auth/                 # API keys, SSH keys (mounted read-only)
├── context/              # Collaboration patterns, project docs (mounted)
├── state/                # Tmux session state, conversation history
├── logs/                 # Agent, supervisor, and MCP server logs
└── tmp/                  # Temporary files, sockets
```

### Volume Mounts

```bash
# Agent container mounts
-v taskspace-project:/workspace                    # Shared project files
-v $HOME/.symposium/auth:/agent/auth:ro           # Authentication secrets
-v $HOME/.symposium/context:/agent/context:ro     # Global context
-v ./config.yaml:/agent/config.yaml:ro            # Per-taskspace config
-v agent-state:/agent/state                       # Persistent agent state
```

## Security Considerations

### Container Isolation

- **Non-root user**: Agent runs as dedicated `agent` user
- **Restricted SSH**: Only pubkey authentication, no root access
- **Limited capabilities**: No privileged operations
- **Resource limits**: CPU and memory constraints

### Secret Management

- **Read-only auth**: API keys mounted read-only from host
- **Key rotation**: Auth directory can be updated without container restart
- **Audit logging**: All agent actions logged to `/agent/logs/`

### Network Security

- **Pod networking**: Only accessible within pod or via SSH
- **Port binding**: SSH port not exposed to host network by default
- **Internal communication**: MCP server only accessible via localhost

This agent container design provides robust, persistent AI agent execution with comprehensive monitoring, recovery, and programmatic control capabilities.