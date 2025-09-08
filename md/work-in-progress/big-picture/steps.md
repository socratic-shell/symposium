# Implementation Steps

This document outlines concrete, actionable steps derived from our experiments and architectural planning.

## Current Focus: Experiment 1 - Containerized Agent

Based on [Experiment 1: Containerized Agent](./experiments/experiment-1-containerized-agent.md), here are the immediate next steps:

### Step 1: Create Basic Dockerfile
**Goal**: Get agent running inside container with SSH access
**Tasks**:
- [ ] Create `containers/agent/Dockerfile` with Ubuntu base
- [ ] Install openssh-server, tmux, nodejs/npm  
- [ ] Install Claude Code CLI in container
- [ ] Configure SSH server with custom entry script
- [ ] Test: `docker run` → SSH connection → agent interaction

**Acceptance**: Can SSH into container and immediately talk to Claude Code

### Step 2: Add File System Integration
**Goal**: Agent can work with mounted project files
**Tasks**:
- [ ] Add volume mount configuration for project directory
- [ ] Configure agent to use `/workspace` as working directory
- [ ] Test file operations: create, edit, delete files through agent
- [ ] Verify changes persist to host filesystem

**Acceptance**: Agent can effectively edit project files, changes visible on host

### Step 3: Secret Management Integration  
**Goal**: Agent can access SSH keys and auth tokens
**Tasks**:
- [ ] Add volume mount for `~/.ssh` directory (read-only)
- [ ] Configure container user/permissions for SSH key access
- [ ] Test: agent can clone repositories using mounted SSH keys
- [ ] Test: agent can push changes to repositories

**Acceptance**: Agent can perform git operations using host credentials

### Step 4: Container Orchestration Basics
**Goal**: Integrate container with existing Symposium infrastructure
**Tasks**:
- [ ] Create container management commands in Symposium daemon
- [ ] Add MCP tools for launching containerized agents
- [ ] Test: spawn container via MCP tool from existing agent
- [ ] Add container status/health monitoring

**Acceptance**: Can launch containerized agent from Symposium UI, monitor its status

## Future Steps (Post-Experiment 1)

### Step 5: Multi-Container Support
- Support running multiple agent containers simultaneously
- Container naming and lifecycle management
- Resource allocation and limits

### Step 6: IDE Integration Patterns
- Test VSCode SSH extension with agent containers
- Explore port forwarding vs SSH remote development
- Compare different IDE connection approaches

### Step 7: Advanced Features
- Agent conversation persistence across container restarts
- Container performance optimization
- Security hardening and user isolation

## Success Metrics

**Short Term (Steps 1-4)**:
- Container startup time < 10 seconds
- Agent response time comparable to native execution
- File operations work seamlessly
- Git operations succeed using host credentials

**Medium Term (Steps 5-7)**:  
- Can run 3+ agent containers simultaneously without performance degradation
- IDE integration feels natural and responsive
- Container management is simple and reliable

## Decision Points

**After Step 2**: Evaluate if file system performance is acceptable. If not, may need to reconsider architecture.

**After Step 3**: Assess security model. If secret management feels too permissive, design more sophisticated injection approach.

**After Step 4**: Decide whether to continue with container approach or return to MVP polish based on experiment results.