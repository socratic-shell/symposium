# Review Format Specification

*This chapter defines the structure and format of review documents.*

## Markdown Structure

Reviews are structured as commit-ready markdown documents with a brief summary followed by detailed context. The default structure optimizes for eventual use as commit messages:

```markdown
# Brief summary of what was implemented

## Context
[Why this change was needed, what goal it serves, background information]

## Changes Made
[Logical walkthrough of what was modified/added]
- Added authentication system ([`src/auth.ts:23`][])
- Updated user model to support login ([`src/models/user.ts:45`][])  
- Integrated auth middleware ([`src/server.ts:67`][])

## Implementation Details
[More detailed explanations of key components and their interactions]

### Authentication Flow ([`src/auth.ts:23`][])
[How the authentication process works...]

### User Model Updates ([`src/models/user.ts:45`][])
[What changes were made and why...]

## Design Decisions
[Rationale for key choices made, alternatives considered]
```

## Code References

Code references use the format `[`file:line`][]` and will be converted to clickable links:
- `[`src/auth.ts:23`][]` - Links to line 23 in the auth module
- `[`README.md:1`][]` - Links to the top of the README

*TODO: Define conventions for referencing ranges, functions, and classes.*

## Default vs Custom Reviews

### Default Structure
The standard format above provides a comprehensive overview suitable for most code changes. It balances commit message utility with detailed technical context.

### Custom Review Styles
Users can request alternative focuses when needed:
- **"Show me the user flow when X happens"** - Trace through specific user journeys
- **"Focus on the architecture decisions"** - Emphasize design choices and trade-offs  
- **"Give me the technical deep-dive"** - Detailed implementation specifics
- **"Walk me through the API changes"** - Focus on interface modifications

The AI assistant should adapt the structure while maintaining the commit-friendly summary and context sections.

*Examples of these variations will be added as we develop usage patterns.*
