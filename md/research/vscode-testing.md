# Testing VSCode Extensions: Official Framework and Best Practices

## Official VSCode testing setup

Microsoft provides an official testing framework for VSCode extensions through **@vscode/test-cli** and **@vscode/test-electron**. This is the recommended approach for extension testing.

### Core philosophy

The official framework launches a real VSCode instance (Extension Development Host) with your extension loaded, allowing you to test against actual VSCode APIs rather than mocks. This ensures your tests match production behavior.

## Installation and setup

### Install dependencies

```bash
npm install --save-dev @vscode/test-cli @vscode/test-electron mocha @types/mocha
```

### Configure test runner

Create `.vscode-test.js` (or `.vscode-test.mjs`) in your project root:

```javascript
// .vscode-test.js
const { defineConfig } = require('@vscode/test-cli');

module.exports = defineConfig({
  files: 'out/test/**/*.test.js',
  version: 'stable', // or 'insiders', '1.75.0', etc.
  workspaceFolder: './test-fixtures',
  mocha: {
    ui: 'tdd',
    timeout: 20000,
    color: true
  },
  // Use env vars to control test behavior
  env: {
    MOCK_CLIENT_MODE: 'true'
  }
});
```

### Project structure

```
your-extension/
├── src/
│   ├── extension.ts
│   ├── clientConnection.ts
│   └── test/
│       ├── runTest.ts         # Optional: custom test runner
│       └── suite/
│           ├── index.ts       # Mocha configuration
│           ├── connection.test.ts
│           ├── panels.test.ts
│           ├── comments.test.ts
│           └── chat.test.ts
├── test-fixtures/             # Test workspace files
│   ├── sample.js
│   └── .vscode/
│       └── settings.json
├── .vscode-test.js
└── package.json
```

### Package.json scripts

```json
{
  "scripts": {
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./",
    "pretest": "npm run compile",
    "test": "vscode-test"
  }
}
```

## Test suite configuration

### Mocha setup (required)

```typescript
// src/test/suite/index.ts
import * as path from 'path';
import * as Mocha from 'mocha';
import { glob } from 'glob';

export function run(): Promise<void> {
  // Create the mocha test
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
    timeout: 10000
  });

  const testsRoot = path.resolve(__dirname, '..');

  return new Promise((resolve, reject) => {
    glob('**/**.test.js', { cwd: testsRoot })
      .then((files) => {
        // Add files to the test suite
        files.forEach(f => mocha.addFile(path.resolve(testsRoot, f)));

        try {
          // Run the mocha test
          mocha.run(failures => {
            if (failures > 0) {
              reject(new Error(`${failures} tests failed.`));
            } else {
              resolve();
            }
          });
        } catch (err) {
          console.error(err);
          reject(err);
        }
      })
      .catch((err) => {
        reject(err);
      });
  });
}
```

## Extension activation in tests

Your extension will be automatically activated when tests run. You can configure this behavior:

```typescript
// src/extension.ts
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
  // Check if running in test mode
  const isTestMode = process.env.MOCK_CLIENT_MODE === 'true';
  
  // Configure based on environment
  const clientConfig = isTestMode 
    ? { useMockClient: true }
    : { useMockClient: false };
  
  // Initialize your extension
  const connection = new ClientConnection(clientConfig);
  
  context.subscriptions.push(connection);
}
```

## Writing integration tests

### Test structure with TDD style

```typescript
// src/test/suite/connection.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Client Connection Tests', () => {
  
  setup(() => {
    // Runs before each test
  });
  
  teardown(() => {
    // Runs after each test
  });
  
  test('Should establish connection', async () => {
    // Your test code
    assert.ok(true);
  });
  
  test('Should handle incoming messages', async () => {
    // Your test code
  });
});
```

### Testing with BDD style (alternative)

```typescript
// .vscode-test.js
module.exports = defineConfig({
  mocha: {
    ui: 'bdd', // Change to BDD
  }
});

// src/test/suite/connection.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

describe('Client Connection Tests', () => {
  
  beforeEach(() => {
    // Setup
  });
  
  afterEach(() => {
    // Teardown
  });
  
  it('should establish connection', async () => {
    // Your test code
  });
});
```

## Testing panel/webview interactions

### What you CAN test

```typescript
// src/test/suite/panels.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Panel Display Tests', () => {
  
  teardown(() => {
    // Always clean up
    vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });
  
  test('Should create webview panel', async () => {
    // Trigger panel creation
    await vscode.commands.executeCommand('yourExtension.showPanel');
    
    // You CAN verify:
    // - That panel was created (track in your extension)
    // - Panel properties (title, viewType)
    // - That webview exists
    
    // You can expose panel state for testing:
    const panels = getExtensionPanels(); // You implement this
    assert.strictEqual(panels.size, 1);
    
    const panel = panels.get('main');
    assert.ok(panel);
    assert.strictEqual(panel.title, 'Expected Title');
    assert.ok(panel.visible);
  });
  
  test('Should communicate with webview via postMessage', async () => {
    await vscode.commands.executeCommand('yourExtension.showPanel');
    
    const panel = getExtensionPanels().get('main');
    
    // Listen for messages from webview
    const messagePromise = new Promise((resolve) => {
      const disposable = panel.webview.onDidReceiveMessage((msg) => {
        disposable.dispose();
        resolve(msg);
      });
    });
    
    // Send message to webview
    panel.webview.postMessage({ command: 'test' });
    
    // Wait for response
    const response = await messagePromise;
    assert.strictEqual(response.status, 'ok');
  });
  
  test('Should update panel on subsequent messages', async () => {
    // Trigger initial panel
    await vscode.commands.executeCommand('yourExtension.showPanel', {
      content: 'Initial'
    });
    await sleep(100);
    
    // Update panel
    await vscode.commands.executeCommand('yourExtension.showPanel', {
      content: 'Updated'
    });
    await sleep(100);
    
    // Verify state updated (requires tracking in extension)
    const panelState = getExtensionPanelState('main');
    assert.strictEqual(panelState.content, 'Updated');
  });
});

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
```

### What you CANNOT test directly

**Critical limitations**:

1. **Cannot access webview DOM**: The webview content is sandboxed and inaccessible from extension tests
2. **Cannot verify visual appearance**: No way to check CSS, layout, or rendered HTML
3. **Cannot simulate user clicks inside webview**: Webview interactions are isolated

**Workaround**: Use message passing between extension and webview to query state:

```typescript
// In your webview HTML/JS
window.addEventListener('message', event => {
  const message = event.data;
  
  if (message.command === 'getState') {
    // Report current state back to extension
    vscode.postMessage({
      command: 'stateResponse',
      data: {
        content: getCurrentContent(),
        buttonEnabled: isButtonEnabled(),
        // ... other state
      }
    });
  }
});

// In your test
test('Should render correct content in webview', async () => {
  const panel = getPanel();
  
  const statePromise = new Promise((resolve) => {
    panel.webview.onDidReceiveMessage((msg) => {
      if (msg.command === 'stateResponse') {
        resolve(msg.data);
      }
    });
  });
  
  // Request state from webview
  panel.webview.postMessage({ command: 'getState' });
  
  const state = await statePromise;
  assert.strictEqual(state.content, 'Expected content');
});
```

## Testing code comments and decorations

### What you CAN test

```typescript
// src/test/suite/comments.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Code Comment Tests', () => {
  let testDoc: vscode.TextDocument;
  let editor: vscode.TextEditor;
  
  setup(async () => {
    // Create test document
    testDoc = await vscode.workspace.openTextDocument({
      content: 'function test() {\n  return 42;\n}\n',
      language: 'javascript'
    });
    editor = await vscode.window.showTextDocument(testDoc);
  });
  
  teardown(async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });
  
  test('Should insert comment into document', async () => {
    // Simulate client message triggering comment insertion
    await vscode.commands.executeCommand('yourExtension.addComment', {
      fileUri: testDoc.uri.toString(),
      line: 1,
      text: '// Generated comment'
    });
    
    await sleep(100);
    
    // You CAN verify: Document text was modified
    const line = testDoc.lineAt(1).text;
    assert.ok(line.includes('// Generated comment'));
  });
  
  test('Should insert multiple comments', async () => {
    const fileUri = testDoc.uri.toString();
    
    await vscode.commands.executeCommand('yourExtension.addComment', {
      fileUri, line: 0, text: '// First'
    });
    
    await vscode.commands.executeCommand('yourExtension.addComment', {
      fileUri, line: 2, text: '// Second'
    });
    
    await sleep(200);
    
    assert.ok(testDoc.lineAt(0).text.includes('// First'));
    assert.ok(testDoc.lineAt(2).text.includes('// Second'));
  });
  
  test('Should handle comment at end of file', async () => {
    const lastLine = testDoc.lineCount - 1;
    
    await vscode.commands.executeCommand('yourExtension.addComment', {
      fileUri: testDoc.uri.toString(),
      line: lastLine,
      text: '// End comment'
    });
    
    await sleep(100);
    
    const text = testDoc.lineAt(lastLine).text;
    assert.ok(text.includes('// End comment'));
  });
});

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
```

### What you CANNOT test directly

**Critical limitation**: VSCode provides **no API to retrieve or inspect decorations**.

When you call `editor.setDecorations(decorationType, ranges)`, there is no corresponding method to get those decorations back.

```typescript
// This does NOT work - no such API exists
const decorations = editor.getDecorations(); // ❌ No such method
const decorations = testDoc.getDecorations(); // ❌ No such method
```

**Workarounds**:

1. **Track decorations in your extension code**:

```typescript
// In your extension
class DecorationManager {
  private static decorations = new Map<string, vscode.TextEditorDecorationType>();
  
  static setDecoration(editor: vscode.TextEditor, type: string, ranges: vscode.Range[]) {
    const decorationType = vscode.window.createTextEditorDecorationType({
      backgroundColor: 'rgba(255, 200, 0, 0.3)'
    });
    
    editor.setDecorations(decorationType, ranges);
    this.decorations.set(type, decorationType);
  }
  
  // Test-only method
  static __testOnly_hasDecoration(type: string): boolean {
    if (process.env.NODE_ENV !== 'test') {
      throw new Error('Test-only method');
    }
    return this.decorations.has(type);
  }
}

// In your test
test('Should apply decoration', async () => {
  await vscode.commands.executeCommand('yourExtension.addComment', {
    fileUri: testDoc.uri.toString(),
    line: 0,
    text: '// Comment'
  });
  
  await sleep(100);
  
  // Can only verify decoration was set, not its appearance
  assert.ok(DecorationManager.__testOnly_hasDecoration('comment'));
});
```

2. **Test decoration logic separately** (unit test):

```typescript
// Unit test the decoration creation logic
suite('Decoration Logic', () => {
  test('Should create correct decoration ranges', () => {
    const ranges = calculateDecorationRanges(/* params */);
    
    assert.strictEqual(ranges.length, 2);
    assert.strictEqual(ranges[0].start.line, 0);
    assert.strictEqual(ranges[0].end.line, 0);
  });
});
```

3. **Use E2E testing** (see section below)

## Testing chat interface

```typescript
// src/test/suite/chat.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Chat Interface Tests', () => {
  
  test('Should send message through extension', async () => {
    // Execute command that sends chat message
    const result = await vscode.commands.executeCommand(
      'yourExtension.sendChatMessage',
      'Test message'
    );
    
    // If your command returns a promise with response:
    assert.ok(result);
    assert.strictEqual(result.status, 'sent');
  });
  
  test('Should handle chat response from client', async () => {
    // This test assumes your mock client (Rust) responds via env var config
    
    const responsePromise = new Promise((resolve) => {
      // Register listener for chat response event
      const disposable = vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('yourExtension.lastChatResponse')) {
          disposable.dispose();
          resolve(vscode.workspace.getConfiguration('yourExtension').get('lastChatResponse'));
        }
      });
    });
    
    await vscode.commands.executeCommand(
      'yourExtension.sendChatMessage',
      'Hello'
    );
    
    const response = await responsePromise;
    assert.ok(response);
  });
  
  test('Should display chat in UI', async () => {
    // Open chat panel
    await vscode.commands.executeCommand('yourExtension.showChat');
    
    const panels = getExtensionPanels();
    assert.ok(panels.has('chat'));
    
    const chatPanel = panels.get('chat');
    assert.strictEqual(chatPanel.title, 'Chat');
    assert.ok(chatPanel.visible);
  });
});
```

## Design patterns for testability

### 1. Expose state for testing

Create a test-only API to access extension internals:

```typescript
// src/extensionState.ts
export class ExtensionState {
  private static panels = new Map<string, vscode.WebviewPanel>();
  private static decorations = new Map<string, any>();
  
  static registerPanel(id: string, panel: vscode.WebviewPanel) {
    this.panels.set(id, panel);
  }
  
  static getPanel(id: string): vscode.WebviewPanel | undefined {
    return this.panels.get(id);
  }
  
  // Test-only methods
  static __testOnly_getAllPanels(): Map<string, vscode.WebviewPanel> {
    if (process.env.NODE_ENV !== 'test') {
      throw new Error('Test-only method');
    }
    return this.panels;
  }
  
  static __testOnly_clearState() {
    if (process.env.NODE_ENV !== 'test') {
      throw new Error('Test-only method');
    }
    this.panels.clear();
    this.decorations.clear();
  }
}
```

### 2. Use environment variables for test configuration

```typescript
// src/extension.ts
export function activate(context: vscode.ExtensionContext) {
  const config = {
    mockMode: process.env.MOCK_CLIENT_MODE === 'true',
    mockDelay: parseInt(process.env.MOCK_DELAY || '100'),
  };
  
  const connection = new ClientConnection(config);
  // ...
}
```

### 3. Create test utilities

```typescript
// src/test/suite/testUtils.ts
import * as vscode from 'vscode';

export async function createTestDocument(
  content: string,
  language: string = 'javascript'
): Promise<vscode.TextDocument> {
  const doc = await vscode.workspace.openTextDocument({
    content,
    language
  });
  return doc;
}

export async function openTestEditor(doc: vscode.TextDocument): Promise<vscode.TextEditor> {
  return await vscode.window.showTextDocument(doc);
}

export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export async function closeAllEditors(): Promise<void> {
  await vscode.commands.executeCommand('workbench.action.closeAllEditors');
}

export async function waitForCondition(
  condition: () => boolean,
  timeout: number = 5000,
  interval: number = 100
): Promise<void> {
  const start = Date.now();
  while (!condition()) {
    if (Date.now() - start > timeout) {
      throw new Error('Timeout waiting for condition');
    }
    await sleep(interval);
  }
}
```

### 4. Always clean up in teardown

```typescript
suite('My Tests', () => {
  let disposables: vscode.Disposable[] = [];
  
  teardown(async () => {
    // Dispose all resources
    disposables.forEach(d => d.dispose());
    disposables = [];
    
    // Close all editors
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
    
    // Clear extension state
    ExtensionState.__testOnly_clearState();
  });
  
  test('Something', async () => {
    const subscription = vscode.workspace.onDidChangeConfiguration(() => {});
    disposables.push(subscription);
    // ...
  });
});
```

## Advanced test runner (optional)

For more control, you can use a custom test runner instead of `vscode-test`:

```typescript
// src/test/runTest.ts
import * as path from 'path';
import { runTests } from '@vscode/test-electron';

async function main() {
  try {
    const extensionDevelopmentPath = path.resolve(__dirname, '../../');
    const extensionTestsPath = path.resolve(__dirname, './suite/index');
    const testWorkspace = path.resolve(__dirname, '../../test-fixtures');
    
    // Download and run VS Code tests
    await runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [
        testWorkspace,
        '--disable-extensions', // Disable other extensions
        '--disable-gpu'         // Better for CI
      ],
      extensionTestsEnv: {
        MOCK_CLIENT_MODE: 'true',
        NODE_ENV: 'test'
      }
    });
  } catch (err) {
    console.error('Failed to run tests');
    console.error(err);
    process.exit(1);
  }
}

main();
```

Then update package.json:

```json
{
  "scripts": {
    "test": "vscode-test",
    "test:custom": "node ./out/test/runTest.js"
  }
}
```

## End-to-end testing with vscode-extension-tester

For scenarios where you **must** verify visual UI state, use **vscode-extension-tester** (built on Selenium):

```bash
npm install --save-dev vscode-extension-tester
```

This framework launches VSCode and automates the UI through Selenium WebDriver, allowing you to:

- Verify webview DOM content
- Check decoration appearance
- Simulate user clicks and interactions
- Verify notification content

**Tradeoffs**:
- Much slower than integration tests (30-60s per test vs 1-2s)
- More brittle (breaks with UI changes)
- Harder to debug
- Requires X server on Linux (xvfb)

**Use sparingly** for critical user workflows only:

```typescript
// src/test/ui/panel.ui.test.ts
import { VSBrowser, WebView, By, until } from 'vscode-extension-tester';

describe('Panel UI Tests', () => {
  let browser: VSBrowser;
  
  before(async function() {
    this.timeout(30000);
    browser = VSBrowser.instance;
  });
  
  it('should display correct content in webview', async function() {
    this.timeout(30000);
    
    // Trigger panel via command palette
    await browser.openResources();
    const input = await browser.openCommandPrompt();
    await input.setText('>Show My Panel');
    await input.confirm();
    
    await browser.driver.sleep(2000);
    
    // Access webview
    const webview = new WebView();
    await webview.switchToFrame();
    
    // Now you can verify DOM
    const heading = await webview.findWebElement(By.css('h1'));
    const text = await heading.getText();
    assert.strictEqual(text, 'Expected Title');
    
    const button = await webview.findWebElement(By.id('myButton'));
    assert.ok(await button.isDisplayed());
    
    await webview.switchBack();
  });
});
```

Run E2E tests:

```bash
npx extest setup-and-run out/test/ui/*.test.js
```

## CI/CD configuration

### GitHub Actions

```yaml
# .github/workflows/test.yml
name: Test Extension

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: actions/setup-node@v4
        with:
          node-version: '18'
      
      - run: npm install
      - run: npm run compile
      
      # Linux requires xvfb for headless VSCode
      - name: Run tests (Linux)
        if: runner.os == 'Linux'
        run: xvfb-run -a npm test
      
      - name: Run tests (Windows/Mac)
        if: runner.os != 'Linux'
        run: npm test
```

### Azure Pipelines

```yaml
# azure-pipelines.yml
trigger:
  - main

strategy:
  matrix:
    linux:
      imageName: 'ubuntu-latest'
    mac:
      imageName: 'macos-latest'
    windows:
      imageName: 'windows-latest'

pool:
  vmImage: $(imageName)

steps:
  - task: NodeTool@0
    inputs:
      versionSpec: '18.x'
  
  - script: npm install
    displayName: 'Install dependencies'
  
  - script: npm run compile
    displayName: 'Compile'
  
  - bash: |
      /usr/bin/Xvfb :99 -screen 0 1024x768x24 > /dev/null 2>&1 &
      echo ">>> Started xvfb"
    displayName: 'Start xvfb'
    condition: eq(variables['Agent.OS'], 'Linux')
  
  - script: npm test
    displayName: 'Run tests'
    env:
      DISPLAY: ':99.0'
```

## Testing best practices summary

### Do's

✅ **Use official @vscode/test-cli framework** - It's the recommended approach  
✅ **Test against real VSCode APIs** - More reliable than mocking  
✅ **Use environment variables** for test configuration  
✅ **Expose test-only state access** methods in your extension  
✅ **Clean up resources** in teardown hooks  
✅ **Wait for async operations** with appropriate delays  
✅ **Test message passing** between extension and webviews  
✅ **Track key state** in your extension for verification  
✅ **Use integration tests** for 80%+ of your testing  
✅ **Test on multiple platforms** in CI/CD

### Don'ts

❌ **Don't try to access webview DOM** in integration tests (use E2E instead)  
❌ **Don't try to retrieve decorations** programmatically (track in extension)  
❌ **Don't forget to close editors** in teardown  
❌ **Don't make tests too slow** (save E2E for critical paths only)  
❌ **Don't test implementation details** - test observable behavior  
❌ **Don't skip CI testing on Windows/Mac** if you support them  
❌ **Don't use sleeps excessively** - prefer event-driven waits when possible

### Recommended test distribution

- **70%** Integration tests (fast, reliable, test API interactions)
- **20%** Unit tests (fastest, test pure logic)
- **10%** E2E tests (slow but comprehensive, test critical UI workflows)

This approach gives you fast, maintainable tests that run against a real VSCode instance while avoiding the limitations of trying to verify visual UI state programmatically.