# SCP Test Runner Architecture

*Scenario-based testing system for Symposium Component Protocol*

## Overview

The SCP test runner allows the same test scenarios to run in different execution contexts. A scenario can be tested with direct ACP protocol communication or through full VSCode integration without changing the test logic.

## Core Concepts

### Test Scenarios

Each test scenario is a directory containing two TypeScript files that define the complete test interaction:

```
symposium/testing/scenarios/
├── basic-echo/
│   ├── mock_agent.ts     # Mock LLM implementation
│   ├── mock_user.ts      # User interaction script
│   └── scenario.json     # Metadata (optional)
└── session-state/
    ├── mock_agent.ts
    ├── mock_user.ts
    └── scenario.json
```

**Key principle**: The same `mock_agent.ts` and `mock_user.ts` files run unchanged across different test execution contexts.

### Test Contexts

Different test runners provide different implementations of the `TestContext` interface:

- **DirectTestContext**: Direct ACP protocol communication (fast, for unit testing)
- **VSCodeTestContext**: Full VSCode integration (slower, for integration testing)
- **Future contexts**: IntelliJ, Zed, or other editor integrations

## Test Scenario Structure

### Mock Agent (`mock_agent.ts`)

The mock agent implements only the LLM behavior and plugs into the real ACP library for protocol handling:

```typescript
#!/usr/bin/env node
import { runMockAgent, AgentContext } from '../../framework/MockAgent.js';

await runMockAgent(async (cx: AgentContext) => {
  await cx.onPrompt(async (message) => {
    if (message === 'Hello, world') {
      return 'Hello, user';
    } else {
      return 'I don\'t understand';
    }
  });
});
```

**Executable requirement**: The agent can be run directly with `node mock_agent.ts`.

**LLM focus**: `createMockAgent()` only mocks the LLM behavior - ACP protocol handling, session management, and message formatting are handled by the real ACP library.

### Mock User (`mock_user.ts`)

The mock user script defines the user interaction sequence using sessions:

```typescript
export async function mockUser(test_cx: TestContext) {
  test_cx.log('info', 'Starting basic echo test');
  
  const session = await test_cx.startSession();
  
  await session.say('Hello, world');
  const response = await session.readResponseString();
  
  if (response !== 'Hello, user') {
    throw new Error(`Expected 'Hello, user', got '${response}'`);
  }
  
  await session.finish();
  test_cx.log('info', 'Basic echo test passed');
}
```

**Context independence**: The user script only depends on the `TestContext` interface.

**Session management**: Tests create sessions, send messages, read responses, and explicitly finish sessions.

## TestContext Interface

The `TestContext` provides a unified API for test interactions:

```typescript
export interface TestContext {
  // Session management
  startSession(): Promise<TestSession>;
  
  // Observability
  log(level: string, message: string): void;
  
  // Cleanup (handles any unfinished sessions)
  finish(): Promise<void>;
}

export interface TestSession {
  // Communication
  say(message: string): Promise<void>;
  readResponseString(): Promise<string>;
  
  // Session lifecycle
  finish(): Promise<void>;
}
```

## Test Runners

### DirectTestRunner

Tests ACP protocol communication by spawning the agent as a subprocess and connecting via stdio:

```typescript
export class DirectTestRunner {
  async runScenario(scenarioPath: string): Promise<void> {
    const { mockUser } = await import(`${scenarioPath}/mock_user.js`);
    
    // Spawn agent subprocess: node mock_agent.ts
    const agentProcess = spawn("node", [`${scenarioPath}/mock_agent.ts`], {
      stdio: ["pipe", "pipe", "inherit"],
    });
    
    // Create ACP client connection to agent's stdio
    const input = Writable.toWeb(agentProcess.stdin!) as WritableStream;
    const output = Readable.toWeb(agentProcess.stdout!) as ReadableStream<Uint8Array>;
    /
    const connection = new ClientSideConnection(
      (_agent) => new TestClient(),
      input,
      output,
    );
    
    // Initialize ACP connection
    await connection.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientCapabilities: { /* ... */ }
    });
    
    const testContext = new DirectTestContext(connection);
    
    try {
      await mockUser(testContext);
    } finally {
      agentProcess.kill();
    }
  }
}
```

**Communication Flow:**
1. **Process spawn**: `node symposium/testing/scenarios/basic-echo/mock_agent.ts` starts ACP server on stdio
2. **ACP connection**: `ClientSideConnection` connects to agent's stdin/stdout streams  
3. **Protocol handshake**: Standard ACP initialization between client and agent
4. **Test execution**: `mockUser()` script creates sessions and sends messages via `TestContext`
5. **Cleanup**: Sessions finished, agent process terminated when test completes

The `DirectTestRunner` uses the `@zed-industries/agent-client-protocol` `ClientSideConnection` class to handle the ACP protocol communication.

**Use cases**:
- Protocol compliance testing
- Agent logic validation

### VSCodeTestRunner

Tests VSCode integration by spawning the agent as a subprocess and communicating through VSCode APIs:

```typescript
export class VSCodeTestRunner {
  async runScenario(scenarioPath: string): Promise<void> {
    const { mockUser } = await import(`${scenarioPath}/mock_user.js`);
    
    // Start agent subprocess via VSCode extension
    await vscode.commands.executeCommand('scp.startAgent', {
      agentPath: `${scenarioPath}/mock_agent.ts`
    });
    
    const testContext = new VSCodeTestContext(/* VSCode APIs */);
    
    try {
      await mockUser(testContext);
    } finally {
      await vscode.commands.executeCommand('scp.stopAgent');
    }
  }
}
```

**Communication Flow:**
1. **VSCode command**: Extension spawns `node mock_agent.ts` and manages ACP connection
2. **Extension bridge**: VSCode extension acts as ACP client, routing messages to/from agent
3. **Test context**: `VSCodeTestContext` uses VSCode commands/events to create sessions and send messages
4. **Integration testing**: Full VSCode ↔ Agent communication path is exercised

**Use cases**:
- Integration testing

## Test Execution

### Direct Testing (Vitest)

```typescript
// tests/scenarios.test.ts
import { describe, it } from 'vitest';
import { DirectTestRunner } from '../src/testing/DirectTestRunner.js';

describe('Direct ACP Tests', () => {
  const runner = new DirectTestRunner();
  
  it('basic-echo', async () => {
    await runner.runScenario('./test-scenarios/basic-echo');
  });
  
  it('session-state', async () => {
    await runner.runScenario('./test-scenarios/session-state');
  });
});
```

**Command**: `npm run test:direct`

### VSCode Integration Testing

```typescript
// src/test/suite/scenarios.test.ts
import * as assert from 'assert';
import { VSCodeTestRunner } from '../../testing/VSCodeTestRunner.js';

suite('VSCode Integration Tests', () => {
  const runner = new VSCodeTestRunner();
  
  test('basic-echo', async () => {
    await runner.runScenario('./test-scenarios/basic-echo');
  });
});
```

**Command**: `npm run test:vscode`

## Implementation Details

### Mock Agent Architecture

The `runMockAgent()` function creates a bridge between a simple LLM function and the full ACP protocol:

```typescript
// Simple usage - initialization and cleanup handled automatically
await runMockAgent(async (cx: AgentContext) => {
  await cx.onPrompt(async (message) => {
    // Simple string in, string out
    return "Hello, user";
  });
});
```

**Integration with ACP library:**
1. `runMockAgent()` creates an `AgentSideConnection` from the ACP library
2. It implements the standard ACP `Agent` interface (`initialize`, `newSession`, `prompt`)
3. The `prompt` handler calls the mock LLM function and formats the response as ACP messages
4. All protocol details (JSON-RPC, session management, message streaming) are handled by the ACP library
5. Initialization and cleanup (including SIGINT handling) are built-in

### TestContext Implementation

The `DirectTestContext` translates high-level test operations into ACP protocol calls:

```typescript
class DirectTestContext implements TestContext {
  constructor(private connection: ClientSideConnection) {}
  
  async startSession(): Promise<TestSession> {
    // Calls ACP newSession request
    const result = await this.connection.newSession({});
    return new DirectTestSession(this.connection, result.sessionId);
  }
}

class DirectTestSession implements TestSession {
  async say(message: string): Promise<void> {
    // Calls ACP prompt request
    await this.connection.prompt({
      sessionId: this.sessionId,
      messages: [{ role: 'user', content: { type: 'text', text: message } }]
    });
  }
  
  async readResponseString(): Promise<string> {
    // Collects sessionUpdate notifications until end_turn
    // Concatenates text chunks into single string
  }
}
```

### Session Lifecycle

**Session creation**: `startSession()` sends ACP `newSession` request and returns session handle

**Message exchange**: `say()` sends ACP `prompt` request, `readResponseString()` collects response chunks

**Session cleanup**: `finish()` can send cancellation if needed, `TestContext.finish()` cleans up any unfinished sessions

**Error handling**: ACP protocol errors are propagated as exceptions, process crashes terminate the test

## Implementation Status

### ✅ Phase 1: Direct Testing Foundation (COMPLETE)

**Implemented:**
- Core abstractions: `TestContext`, `TestSession`, `DirectTestRunner`
- `runMockAgent()` function with built-in initialization and cleanup
- Working `basic-echo` test scenario with real ACP integration
- Vitest integration with `npm run test:direct`
- TypeScript compilation and proper type safety

**Current project structure:**
```
symposium/testing/
├── framework/
│   ├── TestContext.ts      ✅ Session-based testing abstractions
│   ├── DirectTestRunner.ts ✅ ACP subprocess communication
│   ├── MockAgent.ts        ✅ Real ACP library integration
│   └── index.ts           ✅ Framework exports
├── scenarios/
│   └── basic-echo/
│       ├── mock_agent.ts   ✅ Simplified with runMockAgent()
│       └── mock_user.ts    ✅ TestContext-based user script
├── tests/
│   └── scenarios.test.ts   ✅ Vitest integration
├── package.json           ✅ Dependencies and scripts
├── tsconfig.json          ✅ TypeScript configuration
└── vitest.config.ts       ✅ Test configuration
```

**Key architectural decisions made:**
- `runMockAgent()` encapsulates initialization/cleanup (no exposed MockAgent interface)
- `DirectTestContext` created first, then passed to `TestClient` constructor
- Real ACP library integration proven working with `@agentclientprotocol/sdk v0.4.6`

### ✅ Phase 2: VSCode Integration (COMPLETE - EXCEEDED EXPECTATIONS)

**Status:** ✅ COMPLETE with direct ACP SDK integration approach

**Achieved:**
1. **Direct ACP Integration**: VSCode extension directly uses `@agentclientprotocol/sdk` 
2. **Live Chat Interface**: Real-time communication between VSCode chat panel and ACP agents
3. **Modern TypeScript Setup**: TypeScript 5.9.3 + Node.js v22 features without workarounds
4. **Session Management**: Proper ACP session lifecycle with cleanup

**Architecture Breakthrough:** Instead of complex `VSCodeTestContext` approach, we achieved direct SDK integration by resolving TypeScript/Node.js compatibility issues. This provides:
- Simpler architecture (no IPC overhead)
- Better performance (direct communication)
- Easier maintenance (fewer moving parts)
- Real production usage (not just testing)

**Current Status:** The echo agent from `symposium/testing/scenarios/basic-echo/` now works live in VSCode chat panel!

### 🎯 Phase 3: Rich Content Capabilities (NEXT PRIORITY)

**Goal:** Implement SCP's signature features: HTML panels and file comments

**Planned:**
1. **SCP Message Extensions**: Add `_scp/html_panel/show` and `_scp/file_comment/show` support
2. **VSCode UI Integration**: HTML panels in webviews, file comments using VSCode comment API
3. **Walkthrough Generation**: Agent-driven rich content creation
4. **Interactive Testing**: Scenarios that validate rich content display

### 🔮 Phase 4: Advanced Features (FUTURE)

**Planned:**
1. **Multiple Agent Sessions**: Concurrent agent communication
2. **Advanced Error Handling**: Cancellation, timeouts, retry logic  
3. **Performance Optimization**: Connection pooling, message batching
4. **Proxy Chain Architecture**: For complex composition scenarios (if needed)
