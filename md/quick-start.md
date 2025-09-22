# Quick Start

This guide walks you through typical Dialectic workflows.

## Workflow 1: Review Presentation

### 1. Make Code Changes

Work with your AI assistant as usual to make code changes to your project. Enable auto-accept edits to avoid interruptions.

```
You: "Add a user authentication system"
AI: [Makes changes to multiple files]
```

### 2. Request a Review

Ask your AI assistant to present a review of the changes:

```
You: "Present a review of what you just implemented"
```

### 3. View the Review

The review appears in the Dialectic panel in VSCode's sidebar. The review is structured as a markdown document with sections explaining:

- What was implemented and why
- How the code works (narrative walkthrough)
- Key design decisions
- Code references with clickable links

### 4. Navigate the Code

Click on any file:line reference in the review to jump directly to that location in your editor. The references stay current even as you make further changes.

### 5. Continue the Conversation

Discuss the implementation with your AI assistant in the terminal as normal:

```
You: "I think the error handling in the login function could be improved"
AI: "Good point! Let me refactor that and update the review"
```

The review automatically updates to reflect the changes.

## Workflow 2: Ask Symposium

### 1. Select Code

Highlight any code in your VSCode editor - a function, a problematic section, or code you want to discuss.

### 2. Ask Your AI

Right-click and choose **"Ask Symposium"** from the context menu, or use the lightbulb quick action.

### 3. Choose Terminal (if needed)

- **Single AI assistant**: Your message goes directly to that terminal
- **Multiple AI assistants**: A picker shows available options with quick access to your last-used terminal
- **No AI assistants**: You'll see a message about starting an AI session

### 4. Discuss the Code

The selected code appears in your AI terminal with file context. Continue the conversation naturally:

```
AI: "I can see this validation function. The main issue is that it only checks for '@' 
     but doesn't validate email format properly. Here's how we could improve it..."
```

### 5. Request a Broader Review

After discussing specific code, you can ask for a comprehensive review:

```
You: "Now present a review of the entire authentication system"
```

This combines both workflows - targeted code discussion and comprehensive documentation.

## Multi-Window Support

Each VSCode window works independently:
- **Different projects**: Each window can have its own AI assistant
- **Separate reviews**: Reviews appear in the window where the AI is running  
- **Independent terminal selection**: Each window remembers its own preferences

## Tips

- **Use both workflows together**: Ask Symposium for specific questions, review presentation for comprehensive documentation
- **Navigate seamlessly**: Click between review references and continue discussions
- **Iterative refinement**: Reviews evolve as you discuss and improve the code
- **Copy for commits**: Export reviews as commit messages to preserve collaborative context

Dialectic eliminates the friction between coding, discussing, and documenting - creating a seamless collaborative development experience.