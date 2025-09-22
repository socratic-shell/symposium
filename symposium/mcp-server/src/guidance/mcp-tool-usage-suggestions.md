---
name: "MCP Tool Usage Suggestions"
description: "Guidelines for effective use of Symposium's MCP tools"
---

# MCP Tool Usage Suggestions

*Making the most of Symposium's MCP tools for effective collaboration*

## Signal Completion

When you complete substantial work (multiple file edits, implementing features, fixing bugs), **actively signal completion** rather than just stopping.

**Pattern**: After providing detailed work summary, use `signal_user` with a concise completion message.

```
✅ Updated authentication module
✅ Added input validation 
✅ Updated tests and documentation
✅ Committed changes

[Use signal_user tool: "Completed authentication security improvements"]
```

**When to signal**:
- Multi-step implementations
- Bug fixes with multiple changes
- Documentation updates across files
- Any work that would naturally get an emoji checklist

**Message content**: Brief, specific description of what was completed.

## Systematic Code Exploration

Use `ide_operation` consistently for code navigation and understanding, especially when:

**Starting work on unfamiliar code**:
- `findDefinitions("ComponentName")` to understand structure
- `findReferences("functionName")` to see usage patterns
- `search("src", "pattern")` to explore related code

**Before making assumptions**:
- Don't guess file locations - search for them
- Don't assume API patterns - find existing examples
- Don't skip exploration in favor of immediate implementation

**Pattern**: Explore first, then implement
```
1. ide_operation: findDefinitions("AuthToken") 
2. fs_read: Examine the found files
3. ide_operation: search("src", "validate.*token")
4. Now implement with understanding
```

## Tool Selection Principles

**Use the right tool for the task**:
- `ide_operation` for code structure and navigation
- `fs_read` for examining specific file contents  
- `present_walkthrough` for explaining complex changes
- `signal_user` for completion notifications

**Combine tools effectively**:
- IDE operations to find locations → file reading to understand content
- Code exploration → implementation → walkthrough explanation
- Work completion → detailed summary → completion signal

## Common Anti-Patterns

**Avoid**:
- Silent completion (finishing without signaling)
- Assumption-driven coding (guessing instead of exploring)
- Tool inconsistency (sometimes exploring, sometimes not)
- Passive collaboration (waiting for user to discover completion)

**Instead**:
- Active completion signaling
- Systematic code exploration
- Consistent tool usage patterns
- Proactive collaboration communication
