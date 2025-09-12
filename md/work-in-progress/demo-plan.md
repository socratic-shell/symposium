# Symposium Demo Plan

*Target: 5-minute screencast demonstrating Symposium's collaborative AI development workflow*

## Core Concept
**"Symposium developing Symposium"** - A recursive demo showing how I've been dogfooding Symposium to build itself, emphasizing human-AI collaborative partnership over simple automation.

## Demo Flow

### 1. Opening Hook (30 seconds)
- **Setup**: "Let me show you how I built this tool... using this tool"
- **Spawn two taskspaces** with initial prompts:
  - Taskspace 1: Bug fix agent working on Swift code
  - Taskspace 2: Brainstorming/planning agent (like this conversation)

### 2. Philosophy & Parallel Work (1 minute)
- **Core message**: "This is how humans and AI should work together - not command-and-execute, but genuine collaborative partnership"
- **Demonstrate parallel streams**: Show both taskspaces active
- **Complementary strengths**: "I provide vision and judgment, agents provide deep analysis and implementation"

### 3. Walkthrough & Interaction (2 minutes)
- **First milestone**: Bug fix agent completes chunk and presents walkthrough
- **Visual payoff**: Beautiful mermaid diagram showing architectural changes
- **Interactive moment**: Click Reply on a comment - "Hmm, this is curious..."
- **Real dialogue**: Have actual conversation about the implementation
- **Collaborative refinement**: Suggest refactoring, agent implements it
- **Q CLI `/reply` demo**: Show the feature in action during conversation
- **Tool introduction**: "Oh, and this `/reply` feature I just used? I sheepishly admit I implemented that in Q CLI - it's open source, Rust-based. But Symposium is intentionally agnostic - it works with any CLI agent that supports MCP."

### 4. Learning Moment (1 minute)
- **Personal story**: "I've been learning Swift for this project. It's pretty cool, but when I write by hand, it feels clumsy - no muscle memory yet. But I've gotten deep into the semantics through conversation with Claude."
- **Ask Socratic Shell demo**: Highlight SwiftUI annotation (e.g., `@StateObject`, `@EnvironmentObject`)
- **Natural curiosity**: "What does this annotation mean? How does this work?"
- **Instant expertise**: Get clear explanation without breaking flow

### 5. Meta Moment & Wrap-up (30 seconds)
- **Agent interaction**: "Oh hey, did you know you're on live TV? Say hi to the audience!"
- **GitHub tour**: Switch to show open issues, documentation
- **Extensibility vision**: "Currently VSCode only, but designed for any editor - see these IntelliJ and Emacs.app support issues"
- **Call to action**: "I'd love for people to try it out and leave feedback"

## Key Messages

### Technical Architecture
- **MCP-based**: Works with any CLI agent that supports Model Context Protocol
- **Editor agnostic**: VSCode today, but extensible to IntelliJ, Emacs, etc.
- **Intentionally narrow extension points**: Makes adding new editors straightforward

### Collaboration Philosophy
- **Not just automation**: Genuine thinking partnership
- **Complementary strengths**: Human vision + AI analysis/implementation
- **Interactive workflows**: Walkthroughs become conversations
- **Learning-friendly**: AI bridges concept understanding and syntax mastery

### Productivity Claims
- **Dogfooding story**: "My pace of progress has gone up big time"
- **Parallel work streams**: Multiple agents working simultaneously
- **Reduced context switching**: Symposium manages window coordination
- **Iterative improvement**: Work gets better through dialogue

## Working Backwards: Missing Features

### Critical for Demo
- [ ] **Swift bug to demonstrate**: Need a good architectural bug with visual mermaid potential
- [ ] **Walkthrough tool polish**: Ensure mermaid diagrams render beautifully
- [ ] **Ask Socratic Shell integration**: Highlight → reference → query workflow
- [ ] **Reply functionality**: Comments in walkthroughs need reply buttons
- [ ] **Agent personality**: Ensure agents can respond naturally to "say hi to audience"

### Nice to Have
- [ ] **Progress visualization**: Show agent progress in Symposium panel
- [ ] **Window tiling demo**: Automatic layout of multiple VSCode instances
- [ ] **Cross-taskspace references**: Share context between agents
- [ ] **Productivity metrics**: Actual data on development pace improvements

### Post-Demo Roadmap
- [ ] **IntelliJ support**: Extension for JetBrains IDEs
- [ ] **Emacs.app support**: Native macOS Emacs integration
- [ ] **Remote development**: VSCode remote workspace support
- [ ] **Advanced tiling**: Complex window layout management

## Success Metrics
- **Immediate understanding**: Audience grasps the collaborative partnership concept
- **Technical credibility**: Architecture decisions feel thoughtful and extensible
- **Excitement generation**: People want to try it and contribute
- **Clear differentiation**: Not just "AI coding assistant" but "collaborative development orchestration"

## Notes
- Keep energy high - this is exciting technology
- Emphasize authenticity - this is real dogfooding, not a contrived demo
- Show genuine curiosity and learning - makes AI feel approachable
- End with clear ways for people to get involved
