# Continue.dev GUI Integration Guide

**Document Version:** 1.0  
**Based on Continue.dev:** v1.4.46 (October 2025) / main branch  
**Repository:** https://github.com/continuedev/continue  
**Last Updated:** October 9, 2025

---

## Document Overview

This guide explains how to reuse Continue.dev's production-quality chat GUI in your own VS Code extension by implementing their message-passing protocol architecture. Continue.dev was specifically designed with a modular architecture where the GUI communicates with the backend purely through well-defined message protocols, making the GUI genuinely reusable across different implementations.

**Key insight:** Continue.dev already supports both VS Code and JetBrains IDEs using the **same GUI codebase**, proving the architecture's portability.

---

## Architecture Overview

Continue.dev uses a **three-layer message-passing architecture**:

```
┌─────────────────────────────────────┐
│  GUI (React + Redux)                │
│  - Runs in VS Code webview          │
│  - Built separately with Vite       │
│  - Located in gui/ folder           │
└──────────────┬──────────────────────┘
               │ Webview postMessage
┌──────────────┴──────────────────────┐
│  Extension (VS Code API)            │
│  - Hosts webview                    │
│  - Routes messages                  │
│  - Implements IDE interface         │
└──────────────┬──────────────────────┘
               │ JSON-RPC / stdio
┌──────────────┴──────────────────────┐
│  Core (Business Logic)              │
│  - LLM interactions                 │
│  - Configuration                    │
│  - Context providers                │
└─────────────────────────────────────┘
```

**For your use case**, you'd replace Core with your ACP agent:

```
┌─────────────────────────────────────┐
│  Continue GUI (unchanged)           │
└──────────────┬──────────────────────┘
               │ Continue protocols
┌──────────────┴──────────────────────┐
│  YOUR Extension (adapter layer)     │
└──────────────┬──────────────────────┘
               │ Your interface
┌──────────────┴──────────────────────┐
│  YOUR Core → ACP Agent              │
└─────────────────────────────────────┘
```

### Architecture Source Files

- **Codebase layout documentation:** [`continuedev/vscode`](https://hub.continue.dev/continuedev/vscode)
- **Messaging architecture:** Defined in [`core/protocol/`](https://github.com/continuedev/continue/tree/main/core/protocol)
- **GUI components:** [`gui/src/components/`](https://github.com/continuedev/continue/tree/main/gui/src/components)
- **VS Code extension entry:** [`extensions/vscode/src/activation/activate.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/activation/activate.ts)

---

## Protocol Messages

The protocols are defined in `core/protocol/` and consist of four message types:

### 1. **ToWebviewFromIdeProtocol** (Extension → GUI)

Messages the extension sends to the GUI.

**Key protocol file:** [`core/protocol/ide.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/ide.ts)

```typescript
// Common messages you'll need to handle:
{
  messageType: "configUpdate",
  data: { config: SerializedContinueConfig }
}

{
  messageType: "configError", 
  data: { message: string }
}

{
  messageType: "newSessionWithPrompt",
  data: { prompt: string }
}

{
  messageType: "addContextItem",
  data: {
    item: ContextItem,
    historyIndex: number
  }
}

// Streaming LLM response
{
  messageType: "llmStreamChunk",
  data: {
    chunk: ChatMessage,
    index: number
  }
}
```

**References:**
- Protocol definitions: [`core/protocol/index.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/index.ts)
- IDE message types: [`core/protocol/ide.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/ide.ts)

### 2. **ToIdeFromWebviewProtocol** (GUI → Extension)

Messages the GUI sends that you need to handle.

**Key protocol file:** [`core/protocol/webview.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/webview.ts)

```typescript
// User sends a message
{
  messageType: "userInput",
  data: {
    input: string,
    contextItems: ContextItem[]
  }
}

// User selects a context provider (@file, @code, etc.)
{
  messageType: "loadContextProvider",
  data: {
    name: string,
    params: any
  }
}

// User stops generation
{
  messageType: "stopGeneration",
  data: {}
}

// User changes model
{
  messageType: "setModel",
  data: {
    model: string
  }
}

// Request for file contents
{
  messageType: "readRangeInFile",
  data: {
    filepath: string,
    range: { start: number, end: number }
  }
}
```

**References:**
- Webview protocol: [`core/protocol/webview.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/webview.ts)
- Message handling in VS Code: [`extensions/vscode/src/VsCodeExtension.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/VsCodeExtension.ts)

### 3. **ToCoreFromWebviewProtocol** (GUI → Core)

These go through the extension first, so you'd intercept and handle them:

```typescript
{
  messageType: "sendChatMessage",
  data: {
    message: string,
    modelTitle: string
  }
}
```

**References:**
- Core protocol: [`core/protocol/core.ts`](https://github.com/continuedev/continue/blob/main/core/protocol/core.ts)

### 4. **ToWebviewFromCoreProtocol** (Core → GUI)

Messages you'd send from your ACP adapter back to the GUI:

```typescript
// Stream LLM tokens
{
  messageType: "llmStream",
  data: {
    content: string,
    done: boolean,
    index: number
  }
}

// Tool/function call
{
  messageType: "toolCall",
  data: {
    toolCallId: string,
    name: string,
    arguments: object
  }
}
```

---

## Key Data Types

### ContextItem

**Type definition:** [`core/index.d.ts`](https://github.com/continuedev/continue/blob/main/core/index.d.ts)

```typescript
interface ContextItem {
  name: string;
  description: string;
  content: string;
  id?: string;
  editing?: boolean;
  editable?: boolean;
}
```

### ChatMessage

**Type definition:** [`core/index.d.ts`](https://github.com/continuedev/continue/blob/main/core/index.d.ts)

```typescript
interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string | Array<{ type: string; text?: string; imageUrl?: string }>;
  toolCalls?: ToolCall[];
}
```

### SerializedContinueConfig

**Type definition:** [`core/index.d.ts`](https://github.com/continuedev/continue/blob/main/core/index.d.ts)

```typescript
interface SerializedContinueConfig {
  models: Array<{
    title: string;
    provider: string;
    model: string;
    apiKey?: string;
  }>;
  contextProviders?: ContextProviderWithParams[];
  slashCommands?: SlashCommandDescription[];
  // ... many other optional fields
}
```

**Full type definitions:** [`core/index.d.ts`](https://github.com/continuedev/continue/blob/main/core/index.d.ts)

---

## Webview Integration - The Nitty Gritty

### Step 1: Create Webview Panel

**Reference implementation:** [`extensions/vscode/src/ContinueGUIWebviewViewProvider.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/ContinueGUIWebviewViewProvider.ts)

```typescript
import * as vscode from 'vscode';
import * as path from 'path';

export class ContinueGuiProvider implements vscode.WebviewViewProvider {
  private _view?: vscode.WebviewView;

  constructor(
    private readonly _extensionUri: vscode.Uri,
    private readonly _guiDistPath: string
  ) {}

  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken
  ) {
    this._view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.file(this._guiDistPath)
      ]
    };

    // Load the Continue GUI HTML
    webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);

    // Set up message handlers
    this._setupMessageHandlers(webviewView.webview);
  }

  private _getHtmlForWebview(webview: vscode.Webview): string {
    // Path to the Continue GUI dist folder
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.file(path.join(this._guiDistPath, 'assets', 'index.js'))
    );
    const styleUri = webview.asWebviewUri(
      vscode.Uri.file(path.join(this._guiDistPath, 'assets', 'index.css'))
    );

    // Generate nonce for CSP
    const nonce = getNonce();

    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <meta http-equiv="Content-Security-Policy" content="
        default-src 'none'; 
        style-src ${webview.cspSource} 'unsafe-inline';
        script-src 'nonce-${nonce}';
        connect-src https:;
      ">
      <link href="${styleUri}" rel="stylesheet">
      <title>Continue</title>
    </head>
    <body>
      <div id="root"></div>
      <script nonce="${nonce}">
        window.windowId = "${getNonce()}";
        window.serverUrl = "http://localhost:3000"; // Your server if needed
        window.vscMediaUrl = "${webview.asWebviewUri(vscode.Uri.file(this._guiDistPath))}";
        window.ide = "vscode";
      </script>
      <script nonce="${nonce}" src="${scriptUri}"></script>
    </body>
    </html>`;
  }

  private _setupMessageHandlers(webview: vscode.Webview) {
    // Listen for messages FROM the webview
    webview.onDidReceiveMessage(
      async (message) => {
        await this.handleWebviewMessage(message);
      }
    );
  }

  // Send messages TO the webview
  public postMessage(message: any): void {
    this._view?.webview.postMessage(message);
  }

  private async handleWebviewMessage(message: any): Promise<void> {
    const { messageType, data } = message;

    switch (messageType) {
      case "userInput":
        await this.handleUserInput(data);
        break;
        
      case "loadContextProvider":
        await this.handleContextProvider(data);
        break;
        
      case "stopGeneration":
        this.stopCurrentGeneration();
        break;
        
      case "setModel":
        this.setModel(data.model);
        break;

      // ... handle other message types
    }
  }

  private async handleUserInput(data: any): Promise<void> {
    const { input, contextItems } = data;
    
    // This is where you'd call YOUR ACP agent
    // Send the prompt to your ACP agent and stream responses back
    
    // Example: Send to ACP agent
    const response = await this.yourAcpClient.sendMessage({
      prompt: input,
      context: contextItems
    });

    // Stream tokens back to GUI
    for await (const chunk of response) {
      this.postMessage({
        messageType: "llmStreamChunk",
        data: {
          chunk: { role: "assistant", content: chunk },
          index: 0
        }
      });
    }

    // Signal completion
    this.postMessage({
      messageType: "llmStreamChunk",
      data: {
        chunk: { role: "assistant", content: "" },
        index: 0,
        done: true
      }
    });
  }
}

function getNonce(): string {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}
```

**Reference files:**
- Webview provider: [`extensions/vscode/src/ContinueGUIWebviewViewProvider.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/ContinueGUIWebviewViewProvider.ts)
- HTML generation utilities: [`extensions/vscode/src/util/getExtensionUri.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/util/getExtensionUri.ts)

### Step 2: Register in activate()

**Reference implementation:** [`extensions/vscode/src/activation/activate.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/activation/activate.ts)

```typescript
export function activate(context: vscode.ExtensionContext) {
  // Path to Continue's GUI build output
  const guiDistPath = path.join(context.extensionPath, 'continue-gui', 'dist');

  const provider = new ContinueGuiProvider(
    context.extensionUri,
    guiDistPath
  );

  // Register the webview provider
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      'continue.continueGUIView',
      provider,
      {
        webviewOptions: {
          retainContextWhenHidden: true
        }
      }
    )
  );
}
```

### Step 3: package.json Configuration

**Reference:** [`extensions/vscode/package.json`](https://github.com/continuedev/continue/blob/main/extensions/vscode/package.json)

```json
{
  "contributes": {
    "viewsContainers": {
      "activitybar": [
        {
          "id": "continue",
          "title": "Continue",
          "icon": "media/icon.svg"
        }
      ]
    },
    "views": {
      "continue": [
        {
          "type": "webview",
          "id": "continue.continueGUIView",
          "name": "Continue"
        }
      ]
    }
  }
}
```

---

## Building the Continue GUI

Continue's GUI is built separately with Vite.

**Build configuration:** [`gui/vite.config.ts`](https://github.com/continuedev/continue/blob/main/gui/vite.config.ts)

```bash
# In the Continue repo
cd gui
npm install
npm run build

# Output goes to gui/dist
# Copy this to your extension:
cp -r gui/dist your-extension/continue-gui/dist
```

**Build scripts:** [`gui/package.json`](https://github.com/continuedev/continue/blob/main/gui/package.json)

---

## Protocol Adapter Pattern

The key abstraction is creating a thin adapter:

```typescript
class AcpToContinueAdapter {
  constructor(
    private acpClient: YourAcpClient,
    private webviewProvider: ContinueGuiProvider
  ) {}

  async sendChatMessage(message: string, contextItems: ContextItem[]) {
    // Translate Continue format → ACP format
    const acpRequest = {
      prompt: message,
      context: contextItems.map(item => ({
        content: item.content,
        name: item.name
      }))
    };

    // Send to ACP agent
    const stream = await this.acpClient.streamChat(acpRequest);

    // Translate ACP responses → Continue format
    for await (const chunk of stream) {
      this.webviewProvider.postMessage({
        messageType: "llmStreamChunk",
        data: {
          chunk: { role: "assistant", content: chunk.content },
          index: 0
        }
      });
    }
  }
}
```

---

## Source Code Reference Map

### Core Architecture
- **Protocol definitions:** [`core/protocol/`](https://github.com/continuedev/continue/tree/main/core/protocol)
- **Type definitions:** [`core/index.d.ts`](https://github.com/continuedev/continue/blob/main/core/index.d.ts)
- **Core system overview:** [DeepWiki - Core System](https://deepwiki.com/continuedev/continue/2-core-system)

### GUI Layer
- **Main GUI components:** [`gui/src/components/`](https://github.com/continuedev/continue/tree/main/gui/src/components)
- **Chat component:** [`gui/src/pages/gui/Chat.tsx`](https://github.com/continuedev/continue/blob/main/gui/src/pages/gui/Chat.tsx)
- **Layout component:** [`gui/src/components/Layout.tsx`](https://github.com/continuedev/continue/blob/main/gui/src/components/Layout.tsx)
- **Input component:** [`gui/src/components/mainInput/ContinueInputBox.tsx`](https://github.com/continuedev/continue/blob/main/gui/src/components/mainInput/ContinueInputBox.tsx)
- **GUI system overview:** [DeepWiki - GUI System](https://deepwiki.com/continuedev/continue/2.3-gui-system)

### VS Code Extension
- **Extension entry:** [`extensions/vscode/src/activation/activate.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/activation/activate.ts)
- **Webview provider:** [`extensions/vscode/src/ContinueGUIWebviewViewProvider.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/ContinueGUIWebviewViewProvider.ts)
- **VS Code extension class:** [`extensions/vscode/src/VsCodeExtension.ts`](https://github.com/continuedev/continue/blob/main/extensions/vscode/src/VsCodeExtension.ts)
- **Package manifest:** [`extensions/vscode/package.json`](https://github.com/continuedev/continue/blob/main/extensions/vscode/package.json)
- **VS Code extension overview:** [DeepWiki - VS Code Extension](https://deepwiki.com/continuedev/continue/4-vs-code-extension)

### Communication Flow
- **Message passing:** [`core/protocol/messenger/`](https://github.com/continuedev/continue/tree/main/core/protocol/messenger)
- **Communication flow overview:** [DeepWiki - Communication Flow](https://deepwiki.com/continuedev/continue/2.4-communication-flow)

### Context Providers
- **Context providers:** [`core/context/providers/`](https://github.com/continuedev/continue/tree/main/core/context/providers)
- **Context provider docs:** [Continue Docs - Context Providers](https://docs.continue.dev/customization/context-providers)

---

## No Helper Libraries (Yet)

Continue doesn't currently provide a standalone library for hosting their GUI - you need to:

1. **Clone their repo** and build the GUI
2. **Copy the GUI dist** to your extension
3. **Implement the protocol handlers** yourself
4. **Host the webview** using VS Code's standard webview API

The good news: their protocol is well-defined and message-passing makes this clean.

---

## Summary

**What you get:**
- Production-quality React chat UI
- Message history
- Context provider system (@file, @code, etc.)
- Model selection UI
- Stop generation button
- Streaming support

**What you build:**
- ~300-400 lines of adapter code
- Protocol message handlers
- Integration with your ACP client
- Webview hosting boilerplate

**Reusability:** Very high - the GUI is truly decoupled via message passing, just as advertised!

---

## Additional Resources

- **Main Repository:** https://github.com/continuedev/continue
- **Documentation:** https://docs.continue.dev
- **Discord Community:** https://discord.gg/vapESyrFmJ
- **Contributing Guide:** [`CONTRIBUTING.md`](https://github.com/continuedev/continue/blob/main/CONTRIBUTING.md)
- **Architecture Deep Dive:** [DeepWiki Documentation](https://deepwiki.com/continuedev/continue)

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | Oct 9, 2025 | Initial document based on Continue.dev v1.4.46 |

---

**License:** This guide is provided for educational purposes. Continue.dev is licensed under Apache 2.0. See their [LICENSE](https://github.com/continuedev/continue/blob/main/LICENSE) file for details.