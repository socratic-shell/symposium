# Presenting a Walkthrough

Dialectic's review presentation feature transforms AI-generated code explanations into navigable, interactive documentation directly in your VSCode sidebar.

## Requesting a Review

### From Your AI Assistant
Simply ask your AI assistant to present a review of recent changes:

```
"Present a review of the changes you just made"
"Can you walk me through what you implemented?"
"Show me a review of the authentication system"
```

### What Happens
1. **AI generates review**: Structured markdown with explanations and code references
2. **Review appears in VSCode**: Dialectic panel opens in your sidebar
3. **Navigation ready**: Click any code reference to jump to that location

## Review Structure

Reviews typically include:

### Summary Section
High-level overview of what was implemented and why.

### Code Tour
Walkthrough of key implementation details with clickable file references like `[validateUser function](src/auth.ts:42)`.

### Design Decisions
Explanation of architectural choices and trade-offs made.

### Next Steps
Suggestions for future improvements or related work.

## Navigation Features

### Clickable References
- **File links**: `[auth.ts](src/auth.ts)` - Opens the file
- **Line links**: `[auth.ts:42](src/auth.ts#L42)` - Jumps to specific line
- **Range links**: `[auth.ts:42-50](src/auth.ts#L42-L50)` - Highlights line range
- **Search links**: `[validateUser function](src/auth.ts?validateUser)` - Finds pattern in file

### Tree Navigation
The review appears as an expandable tree in your sidebar:
- **Sections** can be collapsed/expanded
- **Code blocks** are syntax highlighted
- **Links** are visually distinct and clickable

## Review Modes

### Replace Mode (Default)
Each new review replaces the previous one, keeping your sidebar clean.

### Update Mode
Updates specific sections of an existing review while preserving others.

### Append Mode
Adds new content to the end of the current review for iterative discussions.

## Working with Reviews

### Iterative Refinement
Continue the conversation with your AI assistant:

```
"The error handling section needs more detail"
"Can you explain the database connection logic better?"
"Add a section about the testing approach"
```

The review updates automatically as your AI assistant refines the explanation.

### Copy for Commits
Use the copy button to export review content as commit messages, preserving the collaborative thinking process in your git history.

### Multi-Window Support
Each VSCode window can have its own review, allowing you to work on multiple projects simultaneously with different AI assistants.

## Best Practices

### Request Specific Reviews
- **"Review the authentication changes"** - Focused on specific functionality
- **"Walk through the API design"** - Architectural overview
- **"Explain the error handling approach"** - Deep dive on specific aspects

### Use During Development
- **After major changes**: Get a walkthrough of what was built
- **Before commits**: Review and document your changes
- **During debugging**: Understand complex code sections
- **For team handoffs**: Create documentation for colleagues

### Combine with Discuss in Symposium
1. **Select confusing code** → Discuss in Symposium → Get explanation
2. **Request broader review** → Get full walkthrough with context
3. **Navigate between** code and documentation seamlessly

Reviews become living documentation that evolves with your codebase, making complex systems easier to understand and maintain.
