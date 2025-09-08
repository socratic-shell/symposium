# Container Orchestration Architecture

This document describes how Symposium manages the lifecycle, networking, and storage for containerized taskspaces using podman pods and advanced volume management techniques.

## Overview

Container orchestration in Symposium handles:
- **Pod Lifecycle**: Creation, startup, monitoring, and cleanup of taskspace pods
- **Volume Management**: Copy-on-write project storage and persistent agent state  
- **Network Coordination**: Pod networking and SSH port management
- **Resource Management**: CPU, memory, and storage limits per taskspace
- **Service Discovery**: Connecting Symposium app to running taskspaces

## Pod Lifecycle Management

### Taskspace Pod Creation

```bash
#!/bin/bash
# /usr/local/bin/create-taskspace-pod

TASKSPACE_ID="$1"
PROJECT_REPO="$2"
BASE_IMAGE="$3"

echo "Creating taskspace pod: $TASKSPACE_ID"

# Create pod with shared networking
podman pod create \
    --name "taskspace-$TASKSPACE_ID" \
    --hostname "$TASKSPACE_ID" \
    --publish 127.0.0.1::2222 \
    --publish 127.0.0.1::2223 \
    --memory=8g \
    --cpus=4

# Get assigned ports
DEV_PORT=$(podman port taskspace-$TASKSPACE_ID-pod 2222 | cut -d: -f2)
AGENT_PORT=$(podman port taskspace-$TASKSPACE_ID-pod 2223 | cut -d: -f2)

echo "Pod created with dev port $DEV_PORT, agent port $AGENT_PORT"

# Store port mapping for later reference
echo "dev_port=$DEV_PORT" > /tmp/symposium-$TASKSPACE_ID/ports
echo "agent_port=$AGENT_PORT" >> /tmp/symposium-$TASKSPACE_ID/ports
```

### Container Startup Coordination

```bash
#!/bin/bash
# /usr/local/bin/start-taskspace-containers

TASKSPACE_ID="$1"
POD_NAME="taskspace-$TASKSPACE_ID"

# Start agent container first (provides IPC daemon)
echo "Starting agent container..."
podman run -d \
    --pod "$POD_NAME" \
    --name "${POD_NAME}-agent" \
    -v "taskspace-$TASKSPACE_ID-project:/workspace" \
    -v "taskspace-$TASKSPACE_ID-agent-state:/agent/state" \
    -v "/tmp/symposium-$TASKSPACE_ID/config.yaml:/agent/config.yaml:ro" \
    -v "$HOME/.symposium/auth:/agent/auth:ro" \
    -v "$HOME/.symposium/context:/agent/context:ro" \
    symposium/agent:latest

# Wait for agent container to be ready
sleep 2

# Start development container  
echo "Starting development container..."
podman run -d \
    --pod "$POD_NAME" \
    --name "${POD_NAME}-dev" \
    -v "taskspace-$TASKSPACE_ID-project:/workspace" \
    -v "taskspace-$TASKSPACE_ID-build-cache:/workspace/.build-cache" \
    symposium/dev-$BASE_IMAGE:latest

echo "Taskspace containers started"
```

### Health Monitoring

```bash
#!/bin/bash
# /usr/local/bin/monitor-taskspace

TASKSPACE_ID="$1"
POD_NAME="taskspace-$TASKSPACE_ID"

while true; do
    # Check container health
    AGENT_STATUS=$(podman inspect "${POD_NAME}-agent" --format='{{.State.Status}}')
    DEV_STATUS=$(podman inspect "${POD_NAME}-dev" --format='{{.State.Status}}')
    
    if [ "$AGENT_STATUS" != "running" ]; then
        echo "Agent container unhealthy: $AGENT_STATUS"
        # Attempt restart
        podman restart "${POD_NAME}-agent"
    fi
    
    if [ "$DEV_STATUS" != "running" ]; then
        echo "Dev container unhealthy: $DEV_STATUS"
        podman restart "${POD_NAME}-dev"
    fi
    
    # Check resource usage
    MEMORY_USAGE=$(podman stats --no-stream --format "table {{.MemUsage}}" "$POD_NAME" | tail -n1)
    echo "Pod memory usage: $MEMORY_USAGE"
    
    sleep 30
done
```

## Volume Management Strategy

### Copy-on-Write Project Storage

Efficient project storage using copy-on-write volumes to minimize disk usage:

```bash
#!/bin/bash  
# /usr/local/bin/setup-cow-project-storage

PROJECT_REPO="$1"
TASKSPACE_ID="$2"

# Create base project volume (shared read-only)
if ! podman volume exists "project-base-$(basename $PROJECT_REPO)"; then
    echo "Creating base project volume..."
    
    # Create temporary container to populate base volume
    podman run --rm \
        -v "project-base-$(basename $PROJECT_REPO):/base" \
        alpine/git:latest \
        clone "$PROJECT_REPO" /base
fi

# Create copy-on-write overlay for this taskspace
echo "Creating CoW overlay volume..."
podman volume create "taskspace-$TASKSPACE_ID-project" \
    --opt "type=overlay" \
    --opt "lowerdir=project-base-$(basename $PROJECT_REPO)" \
    --opt "upperdir=taskspace-$TASKSPACE_ID-changes" \
    --opt "workdir=taskspace-$TASKSPACE_ID-work"
```

### Persistent Agent State

```bash
#!/bin/bash
# /usr/local/bin/setup-agent-persistence

TASKSPACE_ID="$1"

# Create persistent volume for agent state
podman volume create "taskspace-$TASKSPACE_ID-agent-state"

# Initialize agent state structure
podman run --rm \
    -v "taskspace-$TASKSPACE_ID-agent-state:/state" \
    alpine:latest \
    sh -c '
        mkdir -p /state/tmux
        mkdir -p /state/logs  
        mkdir -p /state/history
        chmod 755 /state/*
    '
```

### Build Cache Management

```bash
#!/bin/bash
# /usr/local/bin/setup-build-cache

TASKSPACE_ID="$1" 
PROJECT_TYPE="$2"

# Create build cache volume
podman volume create "taskspace-$TASKSPACE_ID-build-cache"

# Language-specific cache initialization
case "$PROJECT_TYPE" in
    "rust")
        # Rust: target directory and cargo registry
        podman run --rm \
            -v "taskspace-$TASKSPACE_ID-build-cache:/cache" \
            rust:1.75 \
            sh -c 'mkdir -p /cache/target /cache/registry'
        ;;
    "node")
        # Node.js: node_modules and npm cache
        podman run --rm \
            -v "taskspace-$TASKSPACE_ID-build-cache:/cache" \
            node:20 \
            sh -c 'mkdir -p /cache/node_modules /cache/npm'
        ;;
    "python")
        # Python: pip cache and __pycache__
        podman run --rm \
            -v "taskspace-$TASKSPACE_ID-build-cache:/cache" \
            python:3.11 \
            sh -c 'mkdir -p /cache/pip /cache/pycache'
        ;;
esac
```

## Network Management

### Port Allocation Strategy

```bash
#!/bin/bash
# /usr/local/bin/allocate-ports

TASKSPACE_ID="$1"

# Find available port ranges
DEV_PORT=$(shuf -i 10000-20000 -n 1)
AGENT_PORT=$(shuf -i 20001-30000 -n 1)

# Verify ports are available
while netstat -tuln | grep -q ":$DEV_PORT "; do
    DEV_PORT=$(shuf -i 10000-20000 -n 1)
done

while netstat -tuln | grep -q ":$AGENT_PORT "; do
    AGENT_PORT=$(shuf -i 20001-30000 -n 1)
done

echo "Allocated ports: dev=$DEV_PORT, agent=$AGENT_PORT"

# Register port allocation
echo "$TASKSPACE_ID,$DEV_PORT,$AGENT_PORT" >> /var/lib/symposium/port-allocations
```

### SSH Configuration Management

```bash
#!/bin/bash
# /usr/local/bin/update-ssh-config

TASKSPACE_ID="$1"
DEV_PORT="$2"
AGENT_PORT="$3"

# Update user's SSH config
SSH_CONFIG="$HOME/.symposium/ssh_config"

# Remove existing entries for this taskspace
sed -i "/Host $TASKSPACE_ID-/,+4d" "$SSH_CONFIG"

# Add new entries
cat >> "$SSH_CONFIG" << EOF
Host $TASKSPACE_ID-dev
    HostName localhost
    Port $DEV_PORT
    User developer
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
    
Host $TASKSPACE_ID-agent
    HostName localhost  
    Port $AGENT_PORT
    User agent
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null

EOF

echo "SSH configuration updated"
```

### Network Security Policies

```bash
#!/bin/bash
# /usr/local/bin/apply-network-policies

POD_NAME="$1"
SECURITY_LEVEL="$2"

case "$SECURITY_LEVEL" in
    "isolated")
        # No internet access, pod-only networking
        podman network create "isolated-$POD_NAME" --internal
        podman pod restart "$POD_NAME" --network "isolated-$POD_NAME"
        ;;
    "restricted") 
        # Limited internet access, no incoming connections
        iptables -A FORWARD -i "podman-$POD_NAME" -j DROP
        iptables -A FORWARD -o "podman-$POD_NAME" -m state --state ESTABLISHED,RELATED -j ACCEPT
        ;;
    "open")
        # Full internet access (default)
        echo "Using default network policy"
        ;;
esac
```

## Resource Management

### Resource Allocation Policies

```yaml
# /etc/symposium/resource-policies.yaml
taskspace_limits:
  default:
    memory: "4Gi"
    cpu: "2000m"
    storage: "10Gi"
    
  premium:
    memory: "8Gi" 
    cpu: "4000m"
    storage: "50Gi"
    
  minimal:
    memory: "1Gi"
    cpu: "500m"
    storage: "2Gi"

# Per-project overrides
project_overrides:
  "large-monorepo":
    memory: "16Gi"
    cpu: "8000m" 
    storage: "100Gi"
```

### Dynamic Resource Adjustment

```bash
#!/bin/bash
# /usr/local/bin/adjust-resources

POD_NAME="$1"
NEW_MEMORY="$2"
NEW_CPU="$3"

echo "Adjusting resources for $POD_NAME"

# Update pod resource limits
podman update --memory="$NEW_MEMORY" --cpus="$NEW_CPU" "$POD_NAME"

# Log resource change
echo "$(date): $POD_NAME memory=$NEW_MEMORY cpu=$NEW_CPU" >> /var/log/symposium/resource-changes.log
```

### Storage Quota Management

```bash
#!/bin/bash
# /usr/local/bin/manage-storage-quotas

TASKSPACE_ID="$1"
QUOTA_GB="$2"

# Set quota on project volume
podman volume inspect "taskspace-$TASKSPACE_ID-project" --format='{{.Mountpoint}}' | \
    xargs -I {} setquota -u $(id -u) "$QUOTA_GB"G "$QUOTA_GB"G 0 0 {}

echo "Storage quota set: ${QUOTA_GB}GB"
```

## Cleanup and Garbage Collection

### Automatic Cleanup Policies

```bash
#!/bin/bash
# /usr/local/bin/cleanup-inactive-taskspaces

INACTIVE_DAYS="$1"
DRY_RUN="${2:-false}"

echo "Finding taskspaces inactive for $INACTIVE_DAYS days..."

# Find inactive pods
INACTIVE_PODS=$(podman pod ls --format '{{.Name}} {{.Created}}' | \
    awk -v days="$INACTIVE_DAYS" '
    {
        if ($(NF) ~ /days?/ && $(NF-1) >= days) print $1
    }')

for POD in $INACTIVE_PODS; do
    if [[ "$POD" == taskspace-* ]]; then
        TASKSPACE_ID=${POD#taskspace-}
        echo "Found inactive taskspace: $TASKSPACE_ID"
        
        if [ "$DRY_RUN" = "false" ]; then
            # Stop and remove pod
            podman pod stop "$POD"
            podman pod rm "$POD"
            
            # Clean up volumes (keep agent state for potential recovery)
            podman volume rm "taskspace-$TASKSPACE_ID-project" 2>/dev/null || true
            podman volume rm "taskspace-$TASKSPACE_ID-build-cache" 2>/dev/null || true
            
            echo "Cleaned up taskspace: $TASKSPACE_ID"
        else
            echo "Would clean up: $TASKSPACE_ID"
        fi
    fi
done
```

### Volume Garbage Collection

```bash
#!/bin/bash
# /usr/local/bin/cleanup-orphaned-volumes

echo "Cleaning up orphaned volumes..."

# Remove volumes not associated with any taskspace
ORPHANED_VOLUMES=$(podman volume ls --format '{{.Name}}' | \
    grep '^taskspace-' | \
    while read VOLUME; do
        TASKSPACE_ID=$(echo "$VOLUME" | cut -d- -f2)
        if ! podman pod exists "taskspace-$TASKSPACE_ID" 2>/dev/null; then
            echo "$VOLUME"
        fi
    done)

for VOLUME in $ORPHANED_VOLUMES; do
    echo "Removing orphaned volume: $VOLUME"
    podman volume rm "$VOLUME"
done
```

## Service Discovery and Health Monitoring

### Taskspace Registry

```bash
#!/bin/bash
# /usr/local/bin/update-taskspace-registry

TASKSPACE_ID="$1"
STATUS="$2"  # starting, running, stopping, stopped
DEV_PORT="$3"
AGENT_PORT="$4"

REGISTRY_FILE="/var/lib/symposium/taskspace-registry.json"

# Update registry entry
jq --arg id "$TASKSPACE_ID" \
   --arg status "$STATUS" \
   --arg dev_port "$DEV_PORT" \
   --arg agent_port "$AGENT_PORT" \
   --arg timestamp "$(date -Iseconds)" \
   '.[$id] = {
       status: $status,
       dev_port: ($dev_port | tonumber),
       agent_port: ($agent_port | tonumber), 
       last_updated: $timestamp
   }' "$REGISTRY_FILE" > "${REGISTRY_FILE}.tmp" && \
   mv "${REGISTRY_FILE}.tmp" "$REGISTRY_FILE"

echo "Registry updated for $TASKSPACE_ID"
```

### Health Check Integration

```bash
#!/bin/bash
# /usr/local/bin/health-check-all-taskspaces

REGISTRY_FILE="/var/lib/symposium/taskspace-registry.json"

jq -r 'to_entries[] | select(.value.status == "running") | .key' "$REGISTRY_FILE" | \
while read TASKSPACE_ID; do
    echo "Checking health of taskspace: $TASKSPACE_ID"
    
    # Check pod health
    if ! podman pod exists "taskspace-$TASKSPACE_ID"; then
        echo "Pod missing, updating registry..."
        /usr/local/bin/update-taskspace-registry "$TASKSPACE_ID" "stopped" 0 0
        continue
    fi
    
    # Check SSH connectivity
    DEV_PORT=$(jq -r ".\"$TASKSPACE_ID\".dev_port" "$REGISTRY_FILE")
    AGENT_PORT=$(jq -r ".\"$TASKSPACE_ID\".agent_port" "$REGISTRY_FILE")
    
    if ! nc -z localhost "$DEV_PORT"; then
        echo "Dev SSH not responding for $TASKSPACE_ID"
    fi
    
    if ! nc -z localhost "$AGENT_PORT"; then
        echo "Agent SSH not responding for $TASKSPACE_ID" 
    fi
done
```

This orchestration architecture provides robust, scalable container management with efficient resource utilization and comprehensive monitoring capabilities.