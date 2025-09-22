# Message Flows

This section shows the detailed message flows for key Dialectic operations using sequence diagrams.

## Review Presentation Flow

When a user requests a code review, here's the complete message flow:

```mermaid
sequenceDiagram
    participant User
    participant AI as AI Assistant
    participant MCP as MCP Server
    participant Daemon as Daemon Bus
    participant Ext as VSCode Extension
    participant UI as Review Panel

    User->>AI: "Present a review of the changes"
    AI->>MCP: present_review(content, mode, baseUri)
    MCP->>MCP: Discover shell PID
    MCP->>Daemon: PresentReview message + shell PID
    Daemon->>Ext: Broadcast message to all extensions
    Ext->>Ext: Check if shell PID matches terminal
    alt PID matches terminal in this window
        Ext->>UI: Create/update review panel
        Ext->>Daemon: Success response
        Daemon->>MCP: Forward success
        MCP->>AI: Tool execution success
        AI->>User: "Review displayed in VSCode"
    else PID doesn't match
        Ext->>Daemon: Ignore (no response)
        Note over Daemon: Other extensions handle it
    end
```

## Discuss in Symposium Flow

When a user selects code and uses "Discuss in Symposium":

```mermaid
sequenceDiagram
    participant User
    participant Ext as VSCode Extension
    participant Registry as Terminal Registry
    participant Picker as Terminal Picker
    participant Term as Target Terminal
    participant AI as AI Assistant

    User->>Ext: Select code + "Discuss in Symposium"
    Ext->>Registry: Get active terminals
    Registry->>Ext: Return AI-enabled terminal PIDs
    
    alt Single AI terminal
        Ext->>Term: Send formatted message
        Term->>AI: Message appears in terminal
        AI->>User: Responds to code question
    else Multiple AI terminals
        Ext->>Picker: Show terminal selection UI
        Picker->>User: Display terminals with last-used option
        User->>Picker: Select terminal
        Picker->>Ext: Return selected terminal
        Ext->>Term: Send formatted message
        Term->>AI: Message appears in terminal
        AI->>User: Responds to code question
    else No AI terminals
        Ext->>User: Show "no MCP servers" warning
    end
```

## Extension Reload Flow

When a user reloads the VSCode window, the extension restarts but MCP servers continue running:

```mermaid
sequenceDiagram
    participant User
    participant Ext as VSCode Extension
    participant Daemon as Daemon Bus
    participant MCP1 as MCP Server 1
    participant MCP2 as MCP Server 2
    participant Registry as Terminal Registry

    Note over User: Reloads VSCode window (Cmd+R)
    Note over Ext: Extension deactivates/reactivates
    
    Ext->>Daemon: Connect to daemon
    Note over Registry: Registry starts empty
    
    Ext->>Daemon: Marco request (discovery)
    Daemon->>MCP1: Broadcast Marco
    Daemon->>MCP2: Broadcast Marco
    
    MCP1->>Daemon: Polo response (shell PID 12345)
    MCP2->>Daemon: Polo response (shell PID 67890)
    
    Daemon->>Ext: Forward Polo from MCP1
    Daemon->>Ext: Forward Polo from MCP2
    
    Ext->>Registry: Add PID 12345
    Ext->>Registry: Add PID 67890
    
    Note over Ext,Registry: Registry rebuilt, Discuss in Symposium works again
```

## Discovery Protocol (Marco-Polo)

How MCP servers announce their presence and maintain the terminal registry:

```mermaid
sequenceDiagram
    participant MCP as MCP Server
    participant Daemon as Daemon Bus
    participant Ext as VSCode Extension
    participant Registry as Terminal Registry

    Note over MCP: Server starts up
    MCP->>MCP: Discover shell PID via process tree
    MCP->>Daemon: Polo message (shell PID)
    Daemon->>Ext: Broadcast Polo to all extensions
    Ext->>Registry: Add PID to active terminals
    
    Note over MCP,Registry: Server running...
    
    Note over MCP: Server shuts down
    MCP->>Daemon: Goodbye message (shell PID)
    Daemon->>Ext: Broadcast Goodbye to all extensions
    Ext->>Registry: Remove PID from active terminals
```

## Key Message Types

### PresentReview Message
```json
{
  "type": "PresentReview",
  "content": "# Review content...",
  "mode": "replace",
  "baseUri": "/path/to/project",
  "shellPid": 12345
}
```

### Polo Message
```json
{
  "type": "Polo",
  "shellPid": 12345
}
```

### Goodbye Message
```json
{
  "type": "Goodbye", 
  "shellPid": 12345
}
```

These flows show how the daemon message bus enables intelligent routing and multi-window support while maintaining a simple interface for AI assistants.
