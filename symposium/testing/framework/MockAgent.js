import { AgentSideConnection, PROTOCOL_VERSION, ndJsonStream } from '@agentclientprotocol/sdk';
import { Readable, Writable } from 'node:stream';
class MockAgentImpl {
    connection; // Will be set during initialization
    promptHandler;
    sessions = new Map();
    // Remove constructor since connection is set later
    setPromptHandler(handler) {
        this.promptHandler = handler;
    }
    async initialize(params) {
        return {
            protocolVersion: PROTOCOL_VERSION,
            agentCapabilities: {
                loadSession: false,
            },
        };
    }
    async newSession(params) {
        const sessionId = Math.random().toString(36).substring(2);
        this.sessions.set(sessionId, { messages: [] });
        return { sessionId };
    }
    async authenticate(params) {
        return {};
    }
    async setSessionMode(params) {
        return {};
    }
    async prompt(params) {
        if (!this.promptHandler) {
            throw new Error('No prompt handler set');
        }
        const session = this.sessions.get(params.sessionId);
        if (!session) {
            throw new Error(`Session ${params.sessionId} not found`);
        }
        // Extract text from content blocks
        const textBlocks = params.prompt.filter((block) => block.type === 'text');
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
    async cancel(params) {
        // Handle cancellation
    }
}
export async function runMockAgent(llmFn) {
    let agentImpl;
    let connection;
    try {
        console.error('[MockAgent] Starting initialization...');
        // Create connection to stdio
        const output = Writable.toWeb(process.stdout);
        const input = Readable.toWeb(process.stdin);
        console.error('[MockAgent] Created streams, creating AgentSideConnection...');
        agentImpl = new MockAgentImpl();
        connection = new AgentSideConnection((conn) => {
            console.error('[MockAgent] AgentSideConnection callback called');
            agentImpl.connection = conn;
            return agentImpl;
        }, ndJsonStream(output, input));
        console.error('[MockAgent] Created connection, setting up LLM handler...');
        // Set up the LLM handler
        const context = {
            async onPrompt(handler) {
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
    }
    catch (error) {
        console.error('[MockAgent] Error during initialization:', error);
        throw error;
    }
}
