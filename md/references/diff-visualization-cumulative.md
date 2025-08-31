# VSCode diff visualization for cumulative Git changes

Based on comprehensive research of VSCode's extension API, popular Git extensions, and implementation patterns, here's a technical strategy for displaying cumulative Git changes with GitHub-style red/green highlighting in your synthetic PR system.

## Native VSCode diff editor fully supports synthetic content highlighting

The most significant finding is that VSCode's native `vscode.diff` command **does provide** automatic red/green highlighting when given synthetic or virtual file content. This works through the TextDocumentContentProvider API without requiring actual files on disk. The highlighting is automatic - VSCode's diff algorithm detects changes and applies the appropriate theme colors.

### Primary implementation approach using TextDocumentContentProvider

The recommended approach leverages VSCode's proven URI transformation pattern used by the core Git extension:

```typescript
// Register content provider for synthetic cumulative diffs
class CumulativeDiffContentProvider implements vscode.TextDocumentContentProvider {
  private contentMap = new Map<string, string>();
  
  provideTextDocumentContent(uri: vscode.Uri): string {
    const query = JSON.parse(uri.query);
    
    // Generate cumulative content from multiple Git states
    if (query.type === 'base') {
      return this.getBaseContent(query.ref);
    } else if (query.type === 'cumulative') {
      return this.mergeCumulativeChanges(
        query.baseRef,
        query.stagedHunks,
        query.unstagedHunks
      );
    }
    return this.contentMap.get(uri.path) || '';
  }
  
  private mergeCumulativeChanges(baseRef: string, stagedHunks: Hunk[], unstagedHunks: Hunk[]): string {
    // Apply hunks sequentially to create synthetic cumulative state
    let content = this.getBaseContent(baseRef);
    content = this.applyHunks(content, stagedHunks);
    content = this.applyHunks(content, unstagedHunks);
    return content;
  }
}

// Register and use the provider
const provider = new CumulativeDiffContentProvider();
vscode.workspace.registerTextDocumentContentProvider('synthetic-pr', provider);

// Open diff with automatic highlighting
const leftUri = vscode.Uri.parse(`synthetic-pr:base?${JSON.stringify({type: 'base', ref: 'HEAD'})}`);
const rightUri = vscode.Uri.parse(`synthetic-pr:cumulative?${JSON.stringify({
  type: 'cumulative',
  baseRef: 'HEAD',
  stagedHunks: stagedHunks,
  unstagedHunks: unstagedHunks
})}`);

await vscode.commands.executeCommand('vscode.diff', leftUri, rightUri, 'PR Changes: Base → Current');
```

This approach automatically provides:
- **Red background** for removed lines (`diffEditor.removedTextBackground`)
- **Green background** for added lines (`diffEditor.insertedTextBackground`)
- **Character-level highlighting** within changed lines
- **Theme integration** that respects user's color preferences

## Alternative decoration-based approach for enhanced visibility

While decorations cannot be applied directly to diff editors, you can create a custom side-by-side view using regular editors with TextEditorDecorationType for cases where you need more control:

```typescript
const addedLineDecoration = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor('diffEditor.insertedLineBackground'),
  isWholeLine: true,
  overviewRulerColor: new vscode.ThemeColor('diffEditorOverview.insertedForeground'),
  overviewRulerLane: vscode.OverviewRulerLane.Left
});

const removedLineDecoration = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor('diffEditor.removedLineBackground'),
  isWholeLine: true,
  overviewRulerColor: new vscode.ThemeColor('diffEditorOverview.removedForeground'),
  overviewRulerLane: vscode.OverviewRulerLane.Left
});

// Apply to specific line ranges
function applyDiffDecorations(editor: vscode.TextEditor, addedLines: number[], removedLines: number[]) {
  const addedRanges = addedLines.map(line => 
    new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length)
  );
  const removedRanges = removedLines.map(line =>
    new vscode.Range(line, 0, line, editor.document.lineAt(line).text.length)
  );
  
  editor.setDecorations(addedLineDecoration, addedRanges);
  editor.setDecorations(removedLineDecoration, removedRanges);
}
```

## Handling complex multi-state cumulative diffs

Research revealed that T-Mobile's enhanced GitLens fork specifically implements multi-commit aggregation - the exact pattern needed for your synthetic PR system. The key insight is maintaining line number mappings across cumulative changes:

```typescript
class CumulativeHunkProcessor {
  private lineOffsets = new Map<number, number>();
  
  processHunks(baseContent: string, hunks: Hunk[]): ProcessedDiff {
    let currentContent = baseContent.split('\n');
    let cumulativeOffset = 0;
    
    // Sort hunks by line number to apply in order
    const sortedHunks = hunks.sort((a, b) => a.oldStart - b.oldStart);
    
    for (const hunk of sortedHunks) {
      // Adjust hunk line numbers based on cumulative offset
      const adjustedStart = hunk.oldStart + cumulativeOffset;
      
      // Apply hunk changes
      const removed = hunk.oldLines;
      const added = hunk.newLines;
      
      currentContent.splice(
        adjustedStart - 1, 
        removed,
        ...hunk.lines.filter(l => l.startsWith('+')).map(l => l.substring(1))
      );
      
      // Update cumulative offset for subsequent hunks
      cumulativeOffset += (added - removed);
      
      // Track line mappings for proper highlighting
      this.updateLineMapping(adjustedStart, removed, added);
    }
    
    return {
      content: currentContent.join('\n'),
      lineMapping: this.lineOffsets
    };
  }
}
```

## Theme-aware color configuration

VSCode provides semantic theme colors that automatically adapt to light, dark, and high-contrast themes. The key theme variables for diff highlighting are:

```json
{
  "workbench.colorCustomizations": {
    "diffEditor.insertedTextBackground": "#00bb0044",
    "diffEditor.removedTextBackground": "#ff000044",
    "diffEditor.insertedLineBackground": "#00bb0020",
    "diffEditor.removedLineBackground": "#ff000020",
    "diffEditorGutter.insertedLineBackground": "#00bb00aa",
    "diffEditorGutter.removedLineBackground": "#ff0000aa"
  }
}
```

Using `new vscode.ThemeColor('diffEditor.insertedTextBackground')` ensures your extension respects user theme preferences without hardcoding colors.

## Webview-based fallback with diff2html

For scenarios requiring richer visualization than native diff editors provide, implement a webview-based solution using diff2html:

```typescript
class RichDiffViewer {
  private panel: vscode.WebviewPanel;
  
  constructor(context: vscode.ExtensionContext) {
    this.panel = vscode.window.createWebviewPanel(
      'richDiff',
      'PR Changes',
      vscode.ViewColumn.One,
      {
        enableScripts: true,
        localResourceRoots: [context.extensionUri]
      }
    );
  }
  
  async showCumulativeDiff(unifiedDiff: string) {
    const diff2htmlUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'node_modules', 'diff2html', 'bundles', 'js', 'diff2html.min.js')
    );
    
    this.panel.webview.html = `
      <!DOCTYPE html>
      <html>
      <head>
        <link rel="stylesheet" href="${diff2htmlUri.toString().replace('.js', '.css')}">
        <style>
          body { 
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
          }
        </style>
      </head>
      <body>
        <div id="diff-container"></div>
        <script src="${diff2htmlUri}"></script>
        <script>
          const diffHtml = Diff2Html.html(\`${unifiedDiff}\`, {
            drawFileList: true,
            matching: 'lines',
            outputFormat: 'side-by-side',
            synchronisedScroll: true
          });
          document.getElementById('diff-container').innerHTML = diffHtml;
        </script>
      </body>
      </html>
    `;
  }
}
```

## Performance optimization strategies

For large files or numerous changes, implement these optimization techniques proven by popular extensions:

**Decoration type reuse** - Create decoration types once and reuse them throughout the extension lifecycle to avoid memory leaks and improve performance.

**Throttled updates** - When dealing with real-time changes, throttle decoration updates to prevent excessive recomputation:

```typescript
let updateTimeout: NodeJS.Timeout;

function scheduleUpdate(editor: vscode.TextEditor) {
  clearTimeout(updateTimeout);
  updateTimeout = setTimeout(() => updateDecorations(editor), 300);
}
```

**File size limits** - VSCode automatically disables certain features for files over 50MB. Implement similar limits for custom highlighting:

```typescript
const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20MB
if (document.getText().length > MAX_FILE_SIZE) {
  // Fall back to native diff without decorations
  return vscode.commands.executeCommand('vscode.diff', uri1, uri2);
}
```

## Complete implementation strategy

Based on the research findings, here's the recommended implementation path:

### Primary approach: Native diff with synthetic content
1. Use TextDocumentContentProvider to serve cumulative diff content
2. Generate synthetic URIs encoding Git state information
3. Leverage vscode.diff command for automatic highlighting
4. This provides the best performance and native VSCode integration

### Secondary approach: Custom decorations for complex scenarios
1. Use TextEditorDecorationType for cases requiring custom highlighting logic
2. Implement side-by-side views when native diff is insufficient
3. Apply theme-aware colors using VSCode's ThemeColor API

### Tertiary approach: Rich webview for advanced features
1. Implement diff2html-based viewer for GitHub-style rendering
2. Use VSCode CSS variables for theme integration
3. Reserve for cases requiring features beyond native capabilities

This multi-tiered approach ensures your synthetic PR system can handle all scenarios while prioritizing native VSCode integration for the best user experience. The native diff editor with synthetic content provider will cover most use cases with proper red/green highlighting, while the fallback options provide flexibility for edge cases and advanced features.