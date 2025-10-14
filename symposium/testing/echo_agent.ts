#!/usr/bin/env node
import { runMockAgent, AgentContext } from './framework/MockAgent.js';

await runMockAgent(async (cx: AgentContext) => {
  await cx.onPrompt(async (message) => {
    // Echo back the same message
    return message;
  });
});
