import { AgentSideConnection, Agent, PROTOCOL_VERSION, ndJsonStream } from '@agentclientprotocol/sdk';
import * as schema from '@agentclientprotocol/sdk/dist/schema';
import { Readable, Writable } from 'node:stream';

export interface AgentContext {
  onPrompt(handler: (message: string) => Promise<string>): Promise<void>;
}



interface AgentSession {
  messages: string[];
}

class MockAgentImpl implements Agent {
  public connection!: AgentSideConnection; // Will be set during initialization
  private promptHandler?: (message: string) => Promise<string>;
  private sessions = new Map<string, AgentSession>();

  // Remove constructor since connection is set later

  setPromptHandler(handler: (message: string) => Promise<string>) {
    this.promptHandler = handler;
  }

  async initialize(params: schema.InitializeRequest): Promise<schema.InitializeResponse> {
    return {
      protocolVersion: PROTOCOL_VERSION,
      agentCapabilities: {
        loadSession: false,
      },
    };
  }

  async newSession(params: schema.NewSessionRequest): Promise<schema.NewSessionResponse> {
    const sessionId = Math.random().toString(36).substring(2);
    this.sessions.set(sessionId, { messages: [] });
    return { sessionId };
  }

  async authenticate(params: schema.AuthenticateRequest): Promise<schema.AuthenticateResponse | void> {
    return {};
  }

  async setSessionMode(params: schema.SetSessionModeRequest): Promise<schema.SetSessionModeResponse | void> {
    return {};
  }

  async prompt(params: schema.PromptRequest): Promise<schema.PromptResponse> {
    if (!this.promptHandler) {
      throw new Error('No prompt handler set');
    }

    const session = this.sessions.get(params.sessionId);
    if (!session) {
      throw new Error(`Session ${params.sessionId} not found`);
    }

    // Extract text from content blocks
    const textBlocks = params.prompt.filter((block): block is schema.ContentBlock & { type: 'text' } => 
      block.type === 'text'
    );
    const userMessage = textBlocks.map(block => block.text).join(' ');

    // Get response from mock LLM
    const responseText = await this.promptHandler(userMessage);

    // Send response as agent message
    await this.connection.sessionUpdate({
      sessionId: params.sessionId,
      update: {
        sessionUpdate: 'agent_message_chunk',
        content: {
          type: 'text',
          text: responseText,
        },
      },
    });

    return { stopReason: 'end_turn' };
  }

  async cancel(params: schema.CancelNotification): Promise<void> {
    // Handle cancellation
  }
}

export async function runMockAgent(llmFn: (cx: AgentContext) => Promise<void>): Promise<void> {
  let agentImpl: MockAgentImpl;
  let connection: AgentSideConnection;

  try {
    console.error('[MockAgent] Starting initialization...');
    
    // Create connection to stdio
    const output = Writable.toWeb(process.stdout) as WritableStream<Uint8Array>;
    const input = Readable.toWeb(process.stdin) as ReadableStream<Uint8Array>;

    console.error('[MockAgent] Created streams, creating AgentSideConnection...');
    
    agentImpl = new MockAgentImpl();
    
    connection = new AgentSideConnection(
      (conn) => {
        console.error('[MockAgent] AgentSideConnection callback called');
        agentImpl.connection = conn;
        return agentImpl;
      },
      ndJsonStream(output, input)
    );

    console.error('[MockAgent] Created connection, setting up LLM handler...');

    // Set up the LLM handler
    const context: AgentContext = {
      async onPrompt(handler: (message: string) => Promise<string>): Promise<void> {
        console.error('[MockAgent] Setting prompt handler');
        agentImpl.setPromptHandler(handler);
      },
    };

    await llmFn(context);
    console.error('[MockAgent] Initialization complete');

    // Set up cleanup handler
    process.on('SIGINT', () => {
      // Connection cleanup handled by process termination
      process.exit(0);
    });

  } catch (error) {
    console.error('[MockAgent] Error during initialization:', error);
    throw error;
  }
}
