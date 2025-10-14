#!/usr/bin/env node
import { runMockAgent } from './framework/MockAgent.js';
await runMockAgent(async (cx) => {
    await cx.onPrompt(async (message) => {
        // Echo back the same message
        return message;
    });
});
