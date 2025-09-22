# RFC: Exposing IDE capabilities

*A natural language interface to VSCode and Language Server Protocol features*

**Tracking Issue**: [#8](https://github.com/symposium/dialectic/issues/8)

## Problem Statement

Currently, AI assistants working with code need many specific MCP tools to interact with the IDE:
- `dialectic___get_selection` for getting selected text
- `builder_mcp___WorkspaceSearch` for finding code patterns  
- Separate tools would be needed for each LSP feature (find references, go to definition, etc.)

This creates several problems:
- **Tool selection overwhelm**: Too many specific tools make it hard for AI to choose the right approach
- **Inconsistent interfaces**: Each tool has different parameter formats and return structures
- **Limited composability**: Hard to combine operations (e.g., "find references to the currently selected symbol")
- **Poor discoverability**: AI assistants must memorize many tool names and signatures

## Proposed Solution

Replace multiple specific tools with a single `ideCapability(string)` tool that:

1. **Accepts natural language requests**: "find all references to validateToken"
2. **Returns either results or refinement suggestions**: Success with data, or "ambiguous, try one of these options"
3. **Uses a composable JSON mini-language internally** for precise operations
4. **Provides self-teaching error messages** that guide AI assistants toward successful usage

## Interface Design

### Single Entry Point
```typescript
ideCapability(request: string) → string
```

### Response Types

**Success:**
```
"Success, results: [{"file": "auth.ts", "line": 42, "context": "validateToken(user)"}]"
```

**Ambiguous request:**
```
"Ambiguous request, consider one of the following:
(1) {"findReferences": {"symbol": {"name": "validateToken", "file": "auth.ts", "line": 42}}}
(2) {"findReferences": {"symbol": {"name": "validateToken", "file": "utils.ts", "line": 15}}}"
```

**Capability not available:**
```
"We don't have the ability to do that :("
```

## Internal Architecture

The system has three main layers:

### 1. Natural Language Interface
- Converts natural language requests to JSON mini-language programs
- Handles ambiguity resolution and provides refinement suggestions
- Acts as the "front door" for AI assistants

### 2. JSON Mini-Language Runtime
- Executes composable JSON programs
- Manages value types (Symbol, Selection, Location, etc.)
- Handles function composition and error propagation

### 3. VSCode Integration Layer
- Maps JSON functions to actual VSCode/LSP calls
- Handles async operations and editor state
- Returns results in JSON mini-language format

## Benefits

**For AI Assistants:**
- Single tool to learn instead of many specific ones
- Natural language interface reduces cognitive load
- Self-teaching through error messages
- Composable operations enable complex workflows

**For Users:**
- More capable AI assistance with IDE operations
- Consistent interface across all IDE features
- Better error messages and suggestions
- Extensible system for future capabilities

**For Developers:**
- Clean separation between language runtime and IDE integration
- Easy to add new capabilities
- Testable and maintainable architecture
- Reusable across different editors (future)

## Open Questions

This RFC establishes the overall approach, but several design questions need resolution:

1. **[Scripting Language Design](./ide-capabilities/scripting-language.md)**: How should the JSON mini-language work? What are the core concepts and composition rules?

2. **[Natural Language Interface](./ide-capabilities/natural-language-interface.md)**: How do we convert natural language requests to JSON programs? What's the right confidence threshold for execution vs clarification?

3. **[Capability Registry](./ide-capabilities/capability-registry.md)**: What IDE capabilities should we expose initially? What are their function signatures and required value types?

## Implementation Strategy

### Phase 1: Proof of Concept
- Implement basic JSON mini-language runtime
- Create a few essential capabilities (getSelection, findSymbol, findReferences)
- Build simple natural language interface (possibly rule-based)
- Validate the overall approach

### Phase 2: Core Capabilities
- Expand capability set to cover common IDE operations
- Improve natural language processing
- Add comprehensive error handling and suggestions
- Replace existing specific MCP tools

### Phase 3: Advanced Features
- Add refactoring operations (rename, extract method, etc.)
- Integrate with more LSP features
- Optimize performance and user experience
- Consider extending to other editors

## Success Criteria

This RFC will be considered successful when:
- AI assistants can perform common IDE operations through natural language
- The tool selection problem is significantly reduced
- Error messages effectively guide AI assistants to successful usage
- The system is extensible enough to add new capabilities easily
- User feedback indicates improved AI assistance quality

## Next Steps

1. Review and refine this overall proposal
2. Work through the detailed design questions in the sub-RFCs
3. Build a minimal prototype to validate core concepts
4. Iterate based on real usage with AI assistants
