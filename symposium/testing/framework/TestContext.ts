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
