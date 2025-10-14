import { TestContext } from '../../framework/TestContext.js';

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
