- Feature Name: `ide_capabilities`
- Start Date: 2024-12-01
- RFD PR: (leave this empty)
- Socratic Shell Issue: [#8](https://github.com/socratic-shell/dialectic/issues/8)

# Summary

[summary]: #summary

Replace multiple specific IDE integration tools with a single `ideCapability(string)` tool that accepts natural language requests and returns either results or refinement suggestions, using a composable JSON mini-language internally.

# Motivation

[motivation]: #motivation

Currently, AI assistants working with code need many specific MCP tools to interact with the IDE:
- `dialectic___get_selection` for getting selected text
- `builder_mcp___WorkspaceSearch` for finding code patterns  
- Separate tools would be needed for each LSP feature (find references, go to definition, etc.)

This creates several problems:
- **Tool selection overwhelm**: Too many specific tools make it hard for AI to choose the right approach
- **Inconsistent interfaces**: Each tool has different parameter formats and return structures
- **Limited composability**: Hard to combine operations (e.g., "find references to the currently selected symbol")
- **Poor discoverability**: AI assistants must memorize many tool names and signatures

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Instead of learning multiple tools like `get_selection`, `find_references`, `go_to_definition`, AI assistants would use a single tool:

```
ideCapability("find all references to validateToken")
```

The tool responds with either:
- **Success**: `"Success, results: [{"file": "auth.ts", "line": 42, "context": "validateToken(user)"}]"`
- **Ambiguous**: `"Ambiguous request, consider one of the following: (1) validateToken in auth.ts line 42, (2) validateToken in utils.ts line 15"`
- **Not available**: `"We don't have the ability to do that :("` 

This makes IDE operations feel conversational while maintaining precision through the underlying JSON mini-language.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The system has three layers:

## 1. Natural Language Interface
Converts requests like "find references to the selected symbol" into JSON programs:
```json
{"findReferences": {"symbol": {"getSelection": {}}}}
```

## 2. JSON Mini-Language Runtime
Executes composable programs with value types:
- `Symbol`: Represents a code symbol with location
- `Selection`: Current editor selection
- `Location`: File position with context
- Functions: `getSelection()`, `findSymbol(name)`, `findReferences(symbol)`, etc.

## 3. VSCode Integration Layer
Maps JSON functions to actual VSCode/LSP calls and returns structured results.

## Interface
```typescript
ideCapability(request: string) → string
```

Response formats provide either results or guidance for refinement, enabling AI assistants to learn through interaction.

# Drawbacks

[drawbacks]: #drawbacks

- **Complexity**: Natural language parsing adds complexity vs direct tool calls
- **Ambiguity**: Natural language can be imprecise, requiring clarification rounds
- **Performance**: Extra parsing layer may add latency
- **Learning curve**: AI assistants need to learn the natural language patterns

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

**Why this design?**
- Reduces cognitive load on AI assistants (1 tool vs many)
- Enables composition of operations naturally
- Self-teaching through error messages
- Extensible for future IDE capabilities

**Alternatives considered:**
- **Status quo**: Keep adding specific tools - leads to tool proliferation
- **Direct JSON interface**: Skip natural language - harder for AI to use
- **Hybrid approach**: Some specific tools + this system - maintains complexity

**Impact of not doing this:**
- Continued tool proliferation as IDE features are added
- Poor AI assistant experience with IDE operations
- Inconsistent interfaces across capabilities

# Prior art

[prior-art]: #prior-art

- **GitHub Copilot**: Uses natural language for code generation but not IDE operations
- **VSCode Command Palette**: Natural language search for commands, but not composable
- **Language Server Protocol**: Standardized IDE operations, but requires specific tool per operation
- **Cursor IDE**: Natural language interface to some IDE features, but not systematically composable

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- **Natural language processing**: Rule-based vs ML-based approach for parsing requests?
- **JSON mini-language design**: What are the core value types and function signatures?
- **Error handling**: How detailed should error messages be for effective AI learning?
- **Performance**: What's acceptable latency for the natural language processing layer?

# Future possibilities

[future-possibilities]: #future-possibilities

- **Multi-editor support**: Extend beyond VSCode to IntelliJ, Vim, etc.
- **Advanced refactoring**: Support complex multi-step refactoring operations
- **Code generation integration**: Combine with AI code generation for more powerful workflows
- **Team collaboration**: Share and reuse complex IDE operation patterns
- **Plugin ecosystem**: Allow third-party capabilities to be registered with the system
