# Development Environment Container Architecture

The development environment container provides the project-specific toolchain and IDE connectivity, enabling developers to work with the same environment the AI agent uses while maintaining familiar development workflows.

## Overview

The dev container hosts:
- **Project Toolchain**: Language-specific compilers, build tools, runtime environments
- **SSH Access**: Remote IDE connectivity (VSCode, IntelliJ, etc.)
- **Development Services**: Databases, web servers, testing frameworks
- **File System**: Shared project code with agent container

## Base Image Discovery and Building

### Project Configuration Detection

The container building process discovers project requirements through multiple sources:

```bash
#!/bin/bash
# /usr/local/bin/detect-project-config

PROJECT_DIR="$1"
CONFIG_FOUND=""

# Check for explicit container configuration
if [ -f "$PROJECT_DIR/Dockerfile" ]; then
    echo "dockerfile:$PROJECT_DIR/Dockerfile"
    CONFIG_FOUND="dockerfile"
elif [ -f "$PROJECT_DIR/.devcontainer/devcontainer.json" ]; then
    echo "devcontainer:$PROJECT_DIR/.devcontainer/devcontainer.json"
    CONFIG_FOUND="devcontainer"
fi

# Language-specific detection if no explicit config
if [ -z "$CONFIG_FOUND" ]; then
    if [ -f "$PROJECT_DIR/Cargo.toml" ]; then
        echo "language:rust"
    elif [ -f "$PROJECT_DIR/package.json" ]; then
        echo "language:node"
    elif [ -f "$PROJECT_DIR/requirements.txt" ] || [ -f "$PROJECT_DIR/pyproject.toml" ]; then
        echo "language:python"
    elif [ -f "$PROJECT_DIR/go.mod" ]; then
        echo "language:go"
    else
        echo "language:generic"
    fi
fi
```

### Base Image Selection

```yaml
# Language-specific base images
language_bases:
  rust: "rust:1.75"
  node: "node:20"
  python: "python:3.11"
  go: "golang:1.21"
  generic: "ubuntu:22.04"

# Development tool additions per language
dev_tools:
  rust: ["cargo-watch", "rust-analyzer"]
  node: ["nodemon", "typescript"]  
  python: ["pytest", "black", "mypy"]
  go: ["delve", "golangci-lint"]
```

### Container Image Building

**Multi-stage build for efficiency:**

```dockerfile
# Generated Dockerfile for development container
ARG BASE_IMAGE=rust:1.75
FROM ${BASE_IMAGE} as base

# Install common development tools
RUN apt-get update && apt-get install -y \
    openssh-server \
    git \
    curl \
    vim \
    htop \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Language-specific development tools
FROM base as dev-tools
COPY install-dev-tools.sh /tmp/
RUN /tmp/install-dev-tools.sh

# Final development environment
FROM dev-tools as development
RUN useradd -m -s /bin/bash developer && \
    mkdir -p /home/developer/.ssh && \
    chown developer:developer /home/developer/.ssh && \
    chmod 700 /home/developer/.ssh

# SSH configuration for IDE access
COPY sshd_config /etc/ssh/
COPY entrypoint.sh /usr/local/bin/

EXPOSE 2222
CMD ["/usr/local/bin/entrypoint.sh"]
```

## SSH Configuration for IDE Access

### SSH Server Setup

```bash
# /etc/ssh/sshd_config for dev container
Port 2222
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
X11Forwarding yes
AllowTcpForwarding yes

# Development-friendly settings
AcceptEnv LANG LC_*
AcceptEnv TERM
UsePAM yes

# Allow port forwarding for development servers
GatewayPorts clientspecified
AllowStreamLocalForwarding yes
```

### IDE Integration

**VSCode Remote-SSH Configuration:**

```json
# .vscode/settings.json (generated for each taskspace)
{
  "remote.SSH.configFile": "/Users/username/.symposium/ssh_config",
  "remote.SSH.useLocalServer": false,
  "terminal.integrated.defaultProfile.linux": "bash"
}
```

**SSH Config Generation:**

```bash
#!/bin/bash
# /usr/local/bin/generate-ssh-config

TASKSPACE_ID="$1"
DEV_PORT="$2"
AGENT_PORT="$3"

cat >> ~/.symposium/ssh_config << EOF
Host ${TASKSPACE_ID}-dev
    HostName localhost
    Port ${DEV_PORT}
    User developer
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
    
Host ${TASKSPACE_ID}-agent  
    HostName localhost
    Port ${AGENT_PORT}
    User agent
    StrictHostKeyChecking no
    UserKnownHostsFile /dev/null
EOF
```

### Development Workflow Integration

**Port Forwarding for Development Servers:**

```bash
# Automatic port forwarding setup
ssh -L 3000:localhost:3000 taskspace-abc123-dev
ssh -L 5432:localhost:5432 taskspace-abc123-dev  # Database
ssh -L 8080:localhost:8080 taskspace-abc123-dev  # API server
```

## Project Environment Setup

### Dependency Management

```bash
#!/bin/bash
# /usr/local/bin/setup-project-environment

PROJECT_DIR="/workspace"
cd "$PROJECT_DIR"

# Language-specific dependency installation
if [ -f "Cargo.toml" ]; then
    echo "Setting up Rust environment..."
    cargo fetch
    cargo build --release
    
elif [ -f "package.json" ]; then
    echo "Setting up Node.js environment..."
    npm install
    
elif [ -f "requirements.txt" ]; then
    echo "Setting up Python environment..."
    pip install -r requirements.txt
    
elif [ -f "go.mod" ]; then
    echo "Setting up Go environment..."
    go mod download
fi

echo "Project environment ready"
```

### Development Services

**Database Services (when detected):**

```yaml
# Auto-detected service configuration
services:
  postgres:
    condition: "requirements.txt contains psycopg2"
    image: "postgres:15"
    environment:
      POSTGRES_DB: "development"
      POSTGRES_USER: "dev"
      POSTGRES_PASSWORD: "devpass"
    
  redis:
    condition: "package.json dependencies contains redis"
    image: "redis:7"
    
  mongodb:
    condition: "requirements.txt contains pymongo"
    image: "mongo:7"
```

**Service Integration:**

```bash
#!/bin/bash
# /usr/local/bin/start-dev-services

# Start background services based on project requirements
if grep -q "psycopg2" /workspace/requirements.txt 2>/dev/null; then
    echo "Starting PostgreSQL..."
    docker run -d --name postgres-dev \
        -e POSTGRES_DB=development \
        -e POSTGRES_USER=dev \
        -e POSTGRES_PASSWORD=devpass \
        -p 5432:5432 \
        postgres:15
fi

# Wait for services to be ready
sleep 2
```

## File System and Synchronization

### Shared Volume Configuration

The development container shares project files with the agent container through pod volumes:

```bash
# Pod volume creation
podman volume create taskspace-abc123-project

# Mount in both containers
# Agent container:
-v taskspace-abc123-project:/workspace

# Dev container:  
-v taskspace-abc123-project:/workspace
```

### File Watching and Hot Reload

```bash
#!/bin/bash
# /usr/local/bin/watch-project

PROJECT_DIR="/workspace"

# Language-specific file watchers
if [ -f "$PROJECT_DIR/Cargo.toml" ]; then
    # Rust: cargo-watch
    cargo watch -x 'check --tests'
    
elif [ -f "$PROJECT_DIR/package.json" ]; then
    # Node.js: nodemon
    nodemon --watch . --ext js,ts,json
    
elif [ -f "$PROJECT_DIR/requirements.txt" ]; then
    # Python: watchdog
    watchmedo auto-restart --directory=. --pattern="*.py" --recursive
fi
```

### Build Artifact Management

```bash
#!/bin/bash
# /usr/local/bin/manage-build-cache

# Separate build cache volume for efficiency
podman volume create taskspace-abc123-build-cache

# Mount build cache
-v taskspace-abc123-build-cache:/workspace/target     # Rust
-v taskspace-abc123-build-cache:/workspace/node_modules  # Node.js
-v taskspace-abc123-build-cache:/workspace/__pycache__   # Python
```

## Development Tools Integration

### Language Server Protocol (LSP)

```json
# Auto-configured LSP servers per language
{
  "rust": {
    "server": "rust-analyzer",
    "command": ["rust-analyzer"],
    "initialization_options": {
      "cargo": {
        "buildScripts": {
          "enable": true
        }
      }
    }
  },
  "typescript": {
    "server": "typescript-language-server", 
    "command": ["typescript-language-server", "--stdio"]
  },
  "python": {
    "server": "pylsp",
    "command": ["pylsp"]
  }
}
```

### Debugging Configuration

```json
# .vscode/launch.json (auto-generated)
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Debug Rust",
      "type": "cppdbg",
      "request": "launch", 
      "program": "${workspaceFolder}/target/debug/${workspaceFolderBasename}",
      "cwd": "${workspaceFolder}",
      "environment": []
    }
  ]
}
```

### Testing Framework Integration

```bash
#!/bin/bash
# /usr/local/bin/run-tests

cd /workspace

if [ -f "Cargo.toml" ]; then
    cargo test
elif [ -f "package.json" ]; then
    npm test
elif [ -f "requirements.txt" ]; then
    python -m pytest
elif [ -f "go.mod" ]; then
    go test ./...
fi
```

## Container Lifecycle Management

### Entrypoint Script

```bash
#!/bin/bash
# /usr/local/bin/entrypoint.sh

# Set up SSH host keys
ssh-keygen -A

# Set up project environment
/usr/local/bin/setup-project-environment

# Start development services  
/usr/local/bin/start-dev-services &

# Start SSH daemon
/usr/sbin/sshd -D
```

### Health Checks

```bash
#!/bin/bash
# /usr/local/bin/health-check

# Check SSH is responsive
if ! nc -z localhost 2222; then
    echo "SSH not responding"
    exit 1
fi

# Check project dependencies are available
cd /workspace
if [ -f "Cargo.toml" ] && ! cargo check --quiet; then
    echo "Rust environment not ready"
    exit 1
fi

echo "Development container healthy"
```

### Resource Management

```yaml
# Container resource limits
resources:
  limits:
    memory: "4Gi"
    cpu: "2000m"
  requests:
    memory: "1Gi" 
    cpu: "500m"
    
# Volume size limits
volumes:
  project: "10Gi"
  build_cache: "5Gi"
  node_modules: "2Gi"
```

## Security and Access Control

### User Permissions

```bash
# Developer user setup with appropriate permissions
RUN useradd -m -u 1000 -s /bin/bash developer && \
    usermod -aG sudo developer && \
    echo "developer ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers
```

### File System Permissions

```bash
# Ensure proper ownership of mounted volumes
chown -R developer:developer /workspace
chmod 755 /workspace

# Secure SSH configuration
chmod 600 /home/developer/.ssh/authorized_keys
chown developer:developer /home/developer/.ssh/authorized_keys
```

### Network Security

- **Isolated networking**: Container only accessible via SSH or pod networking
- **Port restrictions**: Only development ports (3000, 5432, etc.) forwarded
- **No internet access**: Optional network isolation for sensitive projects

This development container provides a complete, secure, and efficient development environment that seamlessly integrates with both human developers and AI agents.