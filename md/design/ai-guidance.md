# AI Guidance Design Considerations

*This section documents design decisions made specifically to work well with AI collaboration patterns from the socratic shell ecosystem.*

## Collaborative Partnership Model

Dialectic is designed around the socratic shell philosophy of genuine AI-human collaboration rather than command-and-control interactions. This influences several key design decisions:

### Review as Dialogue, Not Report

Traditional code review tools present static snapshots. Dialectic treats reviews as living documents that evolve through conversation:

- **Incremental updates**: The `append` and `update-section` modes allow reviews to grow organically
- **Conversational flow**: Reviews can respond to questions and incorporate new insights
- **Preserved context**: Previous review content remains visible, maintaining conversation history

### Narrative Over Checklist

AI assistants excel at providing narrative explanations rather than mechanical summaries:

- **Story-driven structure**: Reviews explain "how it works" and "why these decisions" 
- **Contextual reasoning**: Design decisions and trade-offs are preserved alongside code
- **Human-readable format**: Markdown optimizes for human understanding, not machine parsing

## File Reference Philosophy

### Rustdoc-Style References

The `[filename:line][]` format was chosen to align with AI assistant natural language patterns:

```markdown
The authentication flow starts in [`src/auth.ts:23`][] and validates tokens using [`src/utils/jwt.ts:45`][].
```

**Design rationale:**
- **Natural integration**: References flow naturally in explanatory text
- **No reference definitions**: AI doesn't need to maintain separate reference sections
- **Familiar syntax**: Similar to rustdoc and other documentation tools AI assistants know

### Semantic Navigation

File references point to semantically meaningful locations, not just changed lines:

- **Function entry points**: Reference where functionality begins, not implementation details
- **Key decision points**: Highlight where important choices are made
- **Interface boundaries**: Show how components connect and communicate

## Tool Interface Design

### Single Focused Tool

Rather than multiple specialized tools, Dialectic provides one flexible `present_review` tool:

**Benefits for AI collaboration:**
- **Cognitive simplicity**: AI assistants can focus on content, not tool selection
- **Flexible modes**: Same tool handles different update patterns naturally
- **Clear purpose**: Unambiguous tool function reduces decision complexity

### Forgiving Parameter Handling

The tool accepts optional parameters and provides sensible defaults:

```typescript
// Minimal usage - just content required
{ content: "# Review content", mode: "replace" }

// Full control when needed
{ content: "...", mode: "update-section", section: "Implementation", baseUri: "/project" }
```

**AI-friendly aspects:**
- **Progressive disclosure**: Simple cases are simple, complex cases are possible
- **Clear error messages**: Validation errors guide AI toward correct usage
- **Flexible content**: No rigid structure requirements for markdown content

## Integration with Socratic Shell Patterns

### Meta Moments

Dialectic supports the socratic shell "meta moment" pattern where collaboration itself becomes a topic:

- **Review evolution**: AI can explain how understanding changed during implementation
- **Process reflection**: Reviews can include notes about the collaborative process
- **Learning capture**: Insights about effective collaboration patterns are preserved

### Beginner's Mind

The system encourages fresh examination rather than pattern matching:

- **No templates**: Reviews aren't forced into rigid structures
- **Contextual adaptation**: Format adapts to what was actually built, not preconceptions
- **Open-ended exploration**: AI can follow interesting threads without constraint

### Persistent Memory

Reviews become part of the project's persistent memory:

- **Commit message integration**: Reviews can become commit messages, preserving reasoning
- **Searchable history**: Past reviews remain accessible for future reference
- **Knowledge accumulation**: Understanding builds over time rather than being lost

## Technical Decisions Supporting AI Collaboration

### Markdown as Universal Format

Markdown was chosen as the review format because:

- **AI native**: Most AI assistants are trained extensively on markdown
- **Human readable**: Developers can read and edit reviews directly
- **Tool agnostic**: Works across different AI assistants and development environments
- **Version controllable**: Reviews can be committed alongside code

### Stateless Tool Design

The `present_review` tool is stateless, requiring no session management:

- **Reliable operation**: Each tool call is independent and self-contained
- **Error recovery**: Failed calls don't corrupt ongoing state
- **Concurrent usage**: Multiple AI assistants could theoretically use the same instance

### Graceful Degradation

The system works even when components fail:

- **Extension offline**: MCP server provides helpful error messages
- **IPC failure**: Clear feedback about connection issues
- **Malformed content**: Security measures prevent crashes while showing errors

## Future AI Integration Opportunities

### Enhanced Code Understanding

The foundation supports future AI-powered features:

- **Semantic file references**: `function:methodName` or `class:ClassName` references
- **Intelligent summarization**: AI could generate section summaries automatically
- **Cross-review connections**: Link related reviews across different changes

### Collaborative Learning

The system could learn from successful collaboration patterns:

- **Review quality metrics**: Track which review styles lead to better outcomes
- **Reference effectiveness**: Learn which file references are most helpful
- **Conversation patterns**: Identify successful dialogue structures

### Multi-AI Coordination

The architecture could support multiple AI assistants:

- **Specialized reviewers**: Different AIs for security, performance, architecture
- **Consensus building**: Multiple perspectives on the same changes
- **Knowledge sharing**: AIs learning from each other's review approaches

These design considerations ensure Dialectic enhances rather than constrains the natural collaborative patterns that emerge between humans and AI assistants.
