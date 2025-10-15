import * as vscode from 'vscode';
import * as path from 'path';
import { spawn, ChildProcess } from 'child_process';
import { Readable, Writable } from 'node:stream';
import type { ReadableStream, WritableStream } from 'node:stream/web';
import { ClientSideConnection, Client, PROTOCOL_VERSION, ndJsonStream } from '@agentclientprotocol/sdk';
import * as schema from '@agentclientprotocol/sdk/dist/schema';

// Simple ACP client for VSCode chat integration
class ChatClient implements Client {
  constructor(private provider: ChatWebviewProvider) {}

  async requestPermission(params: schema.RequestPermissionRequest): Promise<schema.RequestPermissionResponse> {
    return {
      outcome: {
        outcome: 'selected',
        optionId: params.options[0].optionId,
      },
    };
  }

  async sessionUpdate(params: schema.SessionNotification): Promise<void> {
    this.provider.handleSessionUpdate(params);
  }

  async writeTextFile(): Promise<schema.WriteTextFileResponse> {
    throw new Error('writeTextFile not supported in chat');
  }

  async readTextFile(): Promise<schema.ReadTextFileResponse> {
    throw new Error('readTextFile not supported in chat');
  }

  async createTerminal(): Promise<schema.CreateTerminalResponse> {
    throw new Error('createTerminal not supported in chat');
  }

  async terminalOutput(): Promise<schema.TerminalOutputResponse> {
    throw new Error('terminalOutput not supported in chat');
  }

  async releaseTerminal(): Promise<schema.ReleaseTerminalResponse> {
    throw new Error('releaseTerminal not supported in chat');
  }

  async waitForTerminalExit(): Promise<schema.WaitForTerminalExitResponse> {
    throw new Error('waitForTerminalExit not supported in chat');
  }

  async killTerminalCommand(): Promise<schema.KillTerminalResponse> {
    throw new Error('killTerminalCommand not supported in chat');
  }
}

export class ChatWebviewProvider implements vscode.WebviewViewProvider {
  public static readonly viewType = 'symposium.chatView';
  private _view?: vscode.WebviewView;
  private _agentProcess?: ChildProcess;
  private _connection?: ClientSideConnection;
  private _currentSession?: string;

  constructor(private readonly _extensionUri: vscode.Uri) {}

  public resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
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
    try {
      // Start agent if not running
      if (!this._connection) {
        await this._startAgent();
      }

      // Create session if we don't have one
      if (!this._currentSession) {
        const result = await this._connection!.newSession({
          cwd: process.cwd(),
          mcpServers: [],
        });
        this._currentSession = result.sessionId;
      }

      // Send message to agent
      await this._connection!.prompt({
        sessionId: this._currentSession,
        prompt: [
          {
            type: 'text',
            text: userMessage,
          } as schema.ContentBlock,
        ],
      });

      // Response will come via sessionUpdate notifications
    } catch (error) {
      console.error('Error handling user message:', error);
      this._sendToWebview('agentMessage', { text: `Error: ${error}` });
    }
  }

  private async _startAgent(): Promise<void> {
    try {
      // Path to the echo agent
      const agentPath = path.join(__dirname, '../../testing/scenarios/basic-echo/mock_agent.js');
      
      // Spawn agent subprocess
      this._agentProcess = spawn('node', [agentPath], {
        stdio: ['pipe', 'pipe', 'inherit'],
      });

      // Create ACP connection using stdio streams with proper Web Stream conversion
      const stdin = this._agentProcess.stdin!;
      const stdout = this._agentProcess.stdout!;
      
      // Use proper Node.js stream to Web Stream conversion
      const webInput = Writable.toWeb(stdin) as WritableStream<Uint8Array>;
      const webOutput = Readable.toWeb(stdout) as ReadableStream<Uint8Array>;
      
      this._connection = new ClientSideConnection(
        () => new ChatClient(this),
        ndJsonStream(webInput, webOutput)
      );

      // Initialize ACP connection
      await this._connection.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: {
          fs: {
            readTextFile: false,
            writeTextFile: false,
          },
        },
      });

      console.log('ACP agent started successfully');
    } catch (error) {
      console.error('Failed to start ACP agent:', error);
      throw error;
    }
  }

  private _sendToWebview(type: string, data: any): void {
    if (this._view) {
      this._view.webview.postMessage({ type, ...data });
    }
  }

  private _cleanup(): void {
    if (this._connection) {
      this._connection = undefined;
    }
    if (this._agentProcess) {
      this._agentProcess.kill();
      this._agentProcess = undefined;
    }
    this._currentSession = undefined;
  }

  public handleSessionUpdate(params: schema.SessionNotification): void {
    if (params.update.sessionUpdate === 'agent_message_chunk' && 'content' in params.update && params.update.content.type === 'text') {
      this._sendToWebview('agentMessage', { text: params.update.content.text });
    }
  }

  public dispose(): void {
    this._cleanup();
  }
}
