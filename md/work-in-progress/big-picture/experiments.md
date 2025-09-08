# Experiments

This section contains targeted experiments to validate core assumptions and reduce uncertainty in the containerized architecture approach.

## Experiment-Driven Development

Instead of implementing the full orchestration system upfront, we're taking an experimental approach to validate the most uncertain aspects:

1. **Technical Feasibility**: Can the core components work together as envisioned?
2. **User Experience**: Does the containerized approach feel natural and responsive?
3. **Architecture Validation**: Do our design assumptions hold up in practice?

## Current Experiments

### [Experiment 1: HTTP + Buffering Daemon](./experiments/experiment-1-http-buffering-daemon.md)
**Status**: Planning
**Objective**: Evolve the current Unix socket daemon to use HTTP communication and implement message buffering/replay functionality.
**Key Questions**:
- Can we migrate from Unix sockets to HTTP without breaking existing functionality?
- Can we reliably buffer and replay messages when clients disconnect/reconnect?
- Does HTTP performance match Unix socket performance for our use case?
- How do we handle connection state tracking and buffer management?

### [Experiment 2: Containerized Agent](./experiments/experiment-2-containerized-agent.md)
**Status**: Planning (depends on Experiment 1)
**Objective**: Validate that we can run an AI agent inside a container with SSH access and file system integration.
**Key Questions**:
- Can we get an agent running in a container with project files mounted?
- Can we SSH into the container and immediately interact with the agent?
- Does the agent work effectively on mounted files?
- How does IDE connectivity work (SSH extension, remote development, etc.)?

## Future Experiment Ideas

### Experiment 2: Multi-Container Orchestration
Test running multiple agent containers simultaneously with shared project access.

### Experiment 3: IDE Integration Patterns  
Compare different approaches for connecting IDEs to containerized agents (SSH remote, port forwarding, etc.).

### Experiment 4: Secret Management
Validate approaches for injecting authentication tokens, SSH keys, and other secrets.

### Experiment 5: Performance and Resource Usage
Measure container startup times, memory usage, and responsiveness under realistic workloads.