# Discuss in Symposium

Discuss in Symposium lets you send selected code directly to your AI assistant for discussion, creating a seamless bridge between your editor and AI chat sessions.

## How It Works

When you select code in VSCode, Dialectic automatically detects which terminals have active AI assistants and routes your selection to the right place.

## Using Discuss in Symposium

### 1. Select Code
Highlight any code in your VSCode editor - a function, a few lines, or even a single variable.

### 2. Trigger the Action
Right-click and choose **"Discuss in Symposium"** from the context menu, or use the lightbulb quick action that appears.

### 3. Choose Your AI (if needed)
- **Single AI assistant**: Your message goes directly to that terminal
- **Multiple AI assistants**: A picker appears showing available options
- **No AI assistants**: You'll see a helpful message about starting an AI session

### 4. Continue the Conversation
The selected code appears in your AI terminal with context about the file and location. Continue the conversation naturally.

## Terminal Selection

### Smart Detection
Dialectic automatically finds terminals with active AI assistants - no need to name terminals or configure anything.

### Multiple AI Assistants
When you have multiple AI sessions running, the terminal picker shows:
- **Quick access option**: "Use last terminal: [Name]" at the top
- **All available terminals**: Listed with their process IDs
- **Memory**: Your last choice is remembered and highlighted

### Example Picker
```
┌─ Multiple AI-enabled terminals found ─────────────────┐
│ Select terminal for AI chat (first option = quick...) │
├───────────────────────────────────────────────────────┤
│ ● 📜 Use last terminal: Terminal 2    PID: 67890      │
│   Quick access to your previously used terminal       │
│                                                       │
│ ─ All available terminals                             │
│                                                       │
│   Terminal (qterm)           PID: 35059               │
│   Terminal with active MCP server                     │
│                                                       │
│ ⭐ Terminal 2                 PID: 67890 (last used)  │
│   Terminal with active MCP server                     │
└───────────────────────────────────────────────────────┘
```

## Message Format

Your selected code is sent with helpful context:

```
<context>looking at this code from src/auth.ts:42:1-45:20 <content>function validateUser(email) {
  if (!email.includes('@')) {
    return false;
  }
  return true;
}</content></context> can you help me improve this validation?
```

This gives your AI assistant:
- **File location**: `src/auth.ts:42:1-45:20`
- **Actual code**: The selected content
- **Your question**: Added automatically or you can type more

## Tips

- **Select meaningful chunks**: Functions, classes, or logical blocks work best
- **Add your question**: The context menu adds "can you help me improve this?" but you can edit it
- **Use consistently**: The system remembers your preferred terminal for quick access
- **Multiple windows**: Each VSCode window tracks its own AI assistants independently

Discuss in Symposium eliminates the friction of copying code to chat - just select, click, and discuss!
