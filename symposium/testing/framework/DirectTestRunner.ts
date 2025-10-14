import { spawn, ChildProcess } from 'node:child_process';
import { Readable, Writable } from 'node:stream';
import { ClientSideConnection, Client, PROTOCOL_VERSION, ndJsonStream } from '@agentclientprotocol/sdk';
import * as schema from '@agentclientprotocol/sdk/dist/schema';
import { TestContext, TestSession } from './TestContext.js';

class TestClient implements Client {
  constructor(private testContext: DirectTestContext) {}

  async requestPermission(params: schema.RequestPermissionRequest): Promise<schema.RequestPermissionResponse> {
    // Auto-approve all permissions for testing
    return {
      outcome: {
        outcome: 'selected',
        optionId: params.options[0].optionId,
      },
    };
  }

  async sessionUpdate(params: schema.SessionNotification): Promise<void> {
    // Route session updates to the appropriate test session
    this.testContext.handleSessionUpdate(params);
  }

  async writeTextFile(params: schema.WriteTextFileRequest): Promise<schema.WriteTextFileResponse> {
    throw new Error('writeTextFile not implemented in test client');
  }

  async readTextFile(params: schema.ReadTextFileRequest): Promise<schema.ReadTextFileResponse> {
    throw new Error('readTextFile not implemented in test client');
  }

  async createTerminal(params: schema.CreateTerminalRequest): Promise<schema.CreateTerminalResponse> {
    throw new Error('createTerminal not implemented in test client');
  }

  async terminalOutput(params: schema.TerminalOutputRequest): Promise<schema.TerminalOutputResponse> {
    throw new Error('terminalOutput not implemented in test client');
  }

  async releaseTerminal(params: schema.ReleaseTerminalRequest): Promise<schema.ReleaseTerminalResponse> {
    throw new Error('releaseTerminal not implemented in test client');
  }

  async waitForTerminalExit(params: schema.WaitForTerminalExitRequest): Promise<schema.WaitForTerminalExitResponse> {
    throw new Error('waitForTerminalExit not implemented in test client');
  }

  async killTerminalCommand(params: schema.KillTerminalCommandRequest): Promise<schema.KillTerminalResponse> {
    throw new Error('killTerminalCommand not implemented in test client');
  }
}

class DirectTestSession implements TestSession {
  public readonly sessionId: string;
  private connection: ClientSideConnection;
  private responseChunks: string[] = [];
  private responsePromise?: Promise<string>;
  private responseResolve?: (value: string) => void;

  constructor(connection: ClientSideConnection, sessionId: string) {
    this.connection = connection;
    this.sessionId = sessionId;
  }

  async say(message: string): Promise<void> {
    // Start collecting response
    this.responseChunks = [];
    this.responsePromise = new Promise((resolve) => {
      this.responseResolve = resolve;
    });

    // Send prompt with new API
    await this.connection.prompt({
      sessionId: this.sessionId,
      prompt: [
        {
          type: 'text',
          text: message,
        } as schema.ContentBlock,
      ],
    });
  }

  async readResponseString(): Promise<string> {
    if (!this.responsePromise) {
      throw new Error('No message sent - call say() first');
    }
    return this.responsePromise;
  }

  onSessionUpdate(update: schema.SessionNotification['update']): void {
    if (update.sessionUpdate === 'agent_message_chunk' && 'content' in update && update.content.type === 'text') {
      this.responseChunks.push(update.content.text);
      // For now, resolve immediately after first chunk (we can improve this later)
      if (this.responseResolve) {
        this.responseResolve(this.responseChunks.join(''));
        this.responseResolve = undefined;
      }
    }
  }

  async finish(): Promise<void> {
    // Session cleanup if needed
  }
}

class DirectTestContext implements TestContext {
  private connection!: ClientSideConnection;
  private sessions: DirectTestSession[] = [];

  setConnection(connection: ClientSideConnection) {
    this.connection = connection;
  }

  handleSessionUpdate(params: schema.SessionNotification): void {
    // Route to appropriate session
    const session = this.sessions.find(s => s.sessionId === params.sessionId);
    if (session) {
      session.onSessionUpdate(params.update);
    }
  }

  async startSession(): Promise<TestSession> {
    const result = await this.connection.newSession({
      cwd: process.cwd(),
      mcpServers: [],
    });
    const session = new DirectTestSession(this.connection, result.sessionId);
    this.sessions.push(session);
    return session;
  }

  log(level: string, message: string): void {
    console.log(`[${level}] ${message}`);
  }

  async finish(): Promise<void> {
    // Clean up any unfinished sessions
    await Promise.all(this.sessions.map(s => s.finish()));
  }
}

export class DirectTestRunner {
  async runScenario(scenarioPath: string): Promise<void> {
    const { mockUser } = await import(`${scenarioPath}/mock_user.js`);
    
    // Spawn agent subprocess
    const agentProcess = spawn('node', [`${scenarioPath}/mock_agent.js`], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });

    try {
      // Create ACP client connection
      const input = Writable.toWeb(agentProcess.stdin!) as WritableStream<Uint8Array>;
      const output = Readable.toWeb(agentProcess.stdout!) as ReadableStream<Uint8Array>;

      const testContext = new DirectTestContext();
      const connection = new ClientSideConnection(
        () => new TestClient(testContext),
        ndJsonStream(input, output)
      );
      testContext.setConnection(connection);

      // Initialize ACP connection
      await connection.initialize({
        protocolVersion: PROTOCOL_VERSION,
        clientCapabilities: {
          fs: {
            readTextFile: false,
            writeTextFile: false,
          },
        },
      });

      try {
        await mockUser(testContext);
      } finally {
        await testContext.finish();
      }
    } finally {
      agentProcess.kill();
    }
  }
}
