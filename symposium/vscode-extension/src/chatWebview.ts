import * as vscode from 'vscode';
import * as path from 'path';
import { spawn, ChildProcess } from 'child_process';

export class ChatWebviewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symposium.chatView';
  private _view?: vscode.WebviewView;
  private _agentProcess?: ChildProcess;

  constructor(private readonly _extensionUri: vscode.Uri) {}

  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken,
  ) {
    this._view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [this._extensionUri]
    };

    webviewView.webview.html = this._getHtmlForWebview();
    this._setupMessageHandlers();
  }

  public show(): void {
    if (this._view) {
      this._view.show?.(true);
    }
  }

  private _getHtmlForWebview(): string {
    return `<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Symposium Chat</title>
      <style>
        body { 
          font-family: var(--vscode-font-family);
          padding: 10px;
          background: var(--vscode-editor-background);
          color: var(--vscode-editor-foreground);
          margin: 0;
          height: 100vh;
          display: flex;
          flex-direction: column;
        }
        .chat-container {
          flex: 1;
          display: flex;
          flex-direction: column;
          overflow: hidden;
        }
        .messages {
          flex: 1;
          overflow-y: auto;
          padding: 10px 0;
        }
        .message {
          margin: 8px 0;
          padding: 8px;
          border-radius: 4px;
          word-wrap: break-word;
        }
        .user-message {
          background: var(--vscode-button-background);
          margin-left: 20px;
          text-align: right;
        }
        .agent-message {
          background: var(--vscode-input-background);
          margin-right: 20px;
        }
        .input-container {
          display: flex;
          gap: 8px;
          padding: 10px 0;
          border-top: 1px solid var(--vscode-panel-border);
        }
        input {
          flex: 1;
          padding: 8px;
          background: var(--vscode-input-background);
          color: var(--vscode-input-foreground);
          border: 1px solid var(--vscode-input-border);
          border-radius: 3px;
          font-size: 13px;
        }
        button {
          padding: 8px 16px;
          background: var(--vscode-button-background);
          color: var(--vscode-button-foreground);
          border: none;
          border-radius: 3px;
          cursor: pointer;
          font-size: 13px;
        }
        button:hover {
          background: var(--vscode-button-hoverBackground);
        }
      </style>
    </head>
    <body>
      <div class="chat-container">
        <div class="messages" id="messages"></div>
        <div class="input-container">
          <input type="text" id="messageInput" placeholder="Type your message..." />
          <button onclick="sendMessage()">Send</button>
        </div>
      </div>

      <script>
        const vscode = acquireVsCodeApi();
        
        function sendMessage() {
          const input = document.getElementById('messageInput');
          const message = input.value.trim();
          if (message) {
            addMessage(message, 'user');
            vscode.postMessage({ type: 'userMessage', message });
            input.value = '';
          }
        }
        
        function addMessage(text, sender) {
          const messagesDiv = document.getElementById('messages');
          const messageDiv = document.createElement('div');
          messageDiv.className = 'message ' + sender + '-message';
          messageDiv.textContent = text;
          messagesDiv.appendChild(messageDiv);
          messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }
        
        window.addEventListener('message', event => {
          const message = event.data;
          if (message.type === 'agentMessage') {
            addMessage(message.text, 'agent');
          }
        });
        
        document.getElementById('messageInput').addEventListener('keypress', function(e) {
          if (e.key === 'Enter') {
            sendMessage();
          }
        });
      </script>
    </body>
    </html>`;
  }

  private _setupMessageHandlers(): void {
    if (!this._view) return;

    this._view.webview.onDidReceiveMessage(async (message) => {
      switch (message.type) {
        case 'userMessage':
          await this._handleUserMessage(message.message);
          break;
      }
    });
  }

  private async _handleUserMessage(userMessage: string): Promise<void> {
    // For now, just echo back the message
    // TODO: Send to ACP agent
    this._sendToWebview('agentMessage', { text: `Echo: ${userMessage}` });
  }

  private _sendToWebview(type: string, data: any): void {
    if (this._view) {
      this._view.webview.postMessage({ type, ...data });
    }
  }

  private _cleanup(): void {
    if (this._agentProcess) {
      this._agentProcess.kill();
      this._agentProcess = undefined;
    }
  }
}
