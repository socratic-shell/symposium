# Debugging Tools

This chapter covers the debugging tools and information sources available for troubleshooting Socratic Shell issues.

## Debug Command

The `debug dump-messages` command provides real-time insight into IPC message flow through the daemon:

```bash
# Show recent daemon messages (human-readable)
socratic-shell-mcp debug dump-messages

# Show last 10 messages
socratic-shell-mcp debug dump-messages --count 10

# Output as JSON for programmatic processing
socratic-shell-mcp debug dump-messages --json

# Use custom daemon socket prefix (for testing)
socratic-shell-mcp debug dump-messages --prefix test-daemon
```

### Message Format

Debug output shows:
- **Timestamp**: When the message was processed
- **Client Identity**: Type, PID, and working directory context
- **Message Content**: The actual IPC message JSON

Example output:
```
Recent daemon messages (3 of 15 total):
────────────────────────────────────────────────────────────────────────────────
[19:33:43.939] BROADCAST[mcp-server(pid:81332,cwd:…/symposium)] {"type":"taskspace_state",...}
[19:33:44.001] BROADCAST[vscode(pid:12345,cwd:…/my-project)] {"type":"register_taskspace_window",...}
[19:33:44.301] BROADCAST[app(pid:67890,cwd:…/workspace)] {"type":"marco",...}
```

### Client Identity Format

Client identities follow the pattern `prefix(pid:N,cwd:…/path)`:

- **mcp-server**: MCP server processes
- **client**: CLI client (default, customizable with `--identity-prefix`)
- **vscode**: VSCode extension
- **app**: macOS Symposium application

## Log Files

### Development Logging

Enable development logging with the `--dev-log` flag:

```bash
# MCP server with dev logging
socratic-shell-mcp --dev-log

# Client with dev logging  
socratic-shell-mcp client --dev-log

# Daemon with dev logging
socratic-shell-mcp daemon --dev-log
```

Development logs include:
- Actor lifecycle events (spawn, termination)
- Connection establishment and failures
- Message routing decisions
- Error conditions and recovery

### Log Locations

Development logs are written to:
- **macOS**: `~/Library/Logs/socratic-shell/`
- **Linux**: `~/.local/share/socratic-shell/logs/`

Log files are named by component:
- `mcp-server.log`: MCP server process logs
- `daemon.log`: Daemon process logs  
- `client.log`: Client process logs

## Daemon Status

### Process Discovery

Check if the daemon is running:

```bash
# Probe for daemon (exits immediately)
socratic-shell-mcp probe

# Check system processes
ps aux | grep socratic-shell-mcp
```

### Socket Files

The daemon creates Unix domain sockets in `/tmp/`:
- Default: `/tmp/socratic-shell-daemon.sock`
- Custom prefix: `/tmp/{prefix}-daemon.sock`

If socket files exist but connections fail, the daemon process may have crashed. Remove stale socket files manually.

## Common Issues

### "Failed to connect to daemon"

1. Check if daemon is running: `socratic-shell-mcp probe`
2. Try manual daemon start: `socratic-shell-mcp daemon`
3. Check for stale socket files in `/tmp/`
4. Review daemon logs for startup errors

### "No messages in daemon history"

- The daemon only tracks messages while running
- Message history is limited (default: 1000 messages)
- Restart the daemon to clear history

### Client Identity Issues

- Ensure clients use appropriate `--identity-prefix` values
- Check that working directory is accessible
- Verify process has permission to read current directory

## Architecture Notes

The debugging system is built on the RepeaterActor architecture:
- All IPC messages flow through a central repeater
- Message history is maintained in memory
- Client identities are established on connection
- Debug commands query the repeater directly

This centralized design makes it easy to observe all inter-process communication and diagnose message routing issues.
