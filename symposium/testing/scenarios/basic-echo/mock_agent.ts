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
