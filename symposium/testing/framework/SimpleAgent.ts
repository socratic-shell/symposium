// Minimal mock implementation to test our architecture
export interface SimpleAgentContext {
  onPrompt(handler: (message: string) => Promise<string>): void;
}

export function createSimpleAgent(llmFn: (cx: SimpleAgentContext) => Promise<void>) {
  let promptHandler: ((message: string) => Promise<string>) | undefined;

  return {
    async initialize() {
      const context: SimpleAgentContext = {
        onPrompt(handler) {
          promptHandler = handler;
        }
      };
      await llmFn(context);
    },

    async handleMessage(message: string): Promise<string> {
      if (!promptHandler) {
        throw new Error('No prompt handler set');
      }
      return await promptHandler(message);
    }
  };
}
