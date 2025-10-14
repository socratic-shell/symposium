import { describe, it } from 'vitest';
import { DirectTestRunner } from '../framework/DirectTestRunner.js';
import { resolve } from 'node:path';

describe('Direct ACP Tests', () => {
  const runner = new DirectTestRunner();
  
  it('basic-echo', async () => {
    const scenarioPath = resolve('./dist/scenarios/basic-echo');
    await runner.runScenario(scenarioPath);
  });
});
