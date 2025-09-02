# Architectural Blueprint for VSCode Synthetic PR with AI Conversation

## The critical architecture decision

Based on comprehensive research of VSCode APIs, the GitHub PR extension implementation, and AI conversation patterns, the optimal architecture combines **VSCode's native DiffEditor with CommentController API**, enhanced by strategic webview components for AI-specific features. This hybrid approach balances native integration with the flexibility needed for sophisticated AI-human conversations.

## Core API recommendations with implementation guidance

### Comment system architecture uses CommentController exclusively

The research definitively shows that VSCode's CommentController API is the only viable choice for implementing comments. The deprecated DocumentCommentProvider and WorkspaceCommentProvider APIs suffer from performance issues, rigid structure, and limited customization. CommentController, pioneered by the GitHub PR extension in 2018, provides complete control over comment lifecycle with native Comments panel integration.

```typescript
// Initialize comment system
const commentController = vscode.comments.createCommentController(
  'synthetic-pr-ai', 
  'AI Code Review'
);

// Enable commenting across entire document
commentController.commentingRangeProvider = {
  provideCommentingRanges: (document) => {
    return [new vscode.Range(0, 0, document.lineCount - 1, 0)];
  }
};

// Create AI comment thread with custom attribution
const aiThread = commentController.createCommentThread(
  document.uri,
  new vscode.Range(lineNumber, 0, lineNumber, 0),
  []
);
aiThread.contextValue = 'ai-suggestion'; // Enables custom commands
aiThread.label = 'AI Review';
```

The CommentController approach provides **native Comments panel integration**, automatic keyboard navigation, and built-in threading capabilities while maintaining flexibility for AI-specific features through the `contextValue` property and custom commands.

### Diff display leverages built-in DiffEditor with strategic enhancements

The GitHub PR extension's success demonstrates that VSCode's built-in DiffEditor API provides the best foundation for diff display. This approach avoids the complexity and performance overhead of custom webview implementations while maintaining consistency with VSCode's native diff viewing experience.

```typescript
// Display diff using native editor
async function showDiff(originalUri: vscode.Uri, modifiedUri: vscode.Uri, prMetadata: PRData) {
  // Use custom URI scheme for synthetic PR files
  const syntheticOriginal = originalUri.with({ scheme: 'pr-base' });
  const syntheticModified = modifiedUri.with({ scheme: 'pr-head' });
  
  // Register file system provider for synthetic files
  vscode.workspace.registerFileSystemProvider('pr-base', baseFileProvider);
  vscode.workspace.registerFileSystemProvider('pr-head', headFileProvider);
  
  // Open native diff editor
  await vscode.commands.executeCommand(
    'vscode.diff',
    syntheticOriginal,
    syntheticModified,
    `${path.basename(originalUri.fsPath)} - AI Review`
  );
}
```

This approach provides **automatic theme support**, native keyboard shortcuts, accessibility features, and consistent performance even with large files. The limitation of minimal programmatic control over the diff editor is offset by the comment overlay system.

### Hybrid UI strategy for AI conversation features

While core diff and comment functionality uses native APIs, AI-specific features require custom UI elements. The recommended approach implements these through a combination of comment thread customization and a supplementary webview panel for rich AI interactions.

```typescript
class AIReviewPanel {
  private panel: vscode.WebviewPanel;
  private commentController: vscode.CommentController;
  
  constructor(context: vscode.ExtensionContext) {
    // Create side panel for AI conversation context
    this.panel = vscode.window.createWebviewPanel(
      'aiReviewPanel',
      'AI Review Assistant',
      vscode.ViewColumn.Two,
      {
        enableScripts: true,
        retainContextWhenHidden: true
      }
    );
    
    // Handle AI-specific interactions
    this.panel.webview.onDidReceiveMessage(message => {
      switch (message.command) {
        case 'refineAISuggestion':
          this.refineWithContext(message.threadId, message.additionalContext);
          break;
        case 'acceptSuggestion':
          this.applySuggestion(message.threadId);
          break;
      }
    });
  }
  
  // Bridge between webview and comment system
  private async refineWithContext(threadId: string, context: string) {
    const thread = this.findThread(threadId);
    const refinedResponse = await this.aiService.refine(thread, context);
    
    // Add refined suggestion to thread
    thread.comments = [...thread.comments, this.createAIComment(refinedResponse)];
  }
}
```

## Implementation architecture with concrete patterns

### Three-layer architecture for maximum flexibility

The recommended architecture separates concerns into three distinct layers that work together seamlessly:

**Layer 1: Native VSCode Integration**
- DiffEditor for file comparison display
- CommentController for threading and basic comments
- Standard VSCode commands for navigation
- Native Comments panel for conversation overview

**Layer 2: AI Service Layer**
- Language model integration for code analysis
- Context management for multi-file awareness
- Suggestion generation with confidence scoring
- Iterative refinement based on human feedback

**Layer 3: Enhanced UI Components**
- Webview panel for AI conversation controls
- Custom comment rendering for AI suggestions
- Context selection interface for focused analysis
- Visual indicators for suggestion types and confidence

### Comment thread management for AI conversations

The implementation distinguishes between AI and human comments through careful thread and comment structuring:

```typescript
interface AIComment extends vscode.Comment {
  body: vscode.MarkdownString;
  mode: vscode.CommentMode;
  author: vscode.CommentAuthorInformation;
  contextValue: 'ai-suggestion' | 'ai-refinement';
  metadata?: {
    confidence: 'high' | 'medium' | 'low';
    type: 'security' | 'performance' | 'style' | 'logic';
    reasoning?: string;
  };
}

class AICommentBuilder {
  createAISuggestion(analysis: AIAnalysis): AIComment {
    const body = new vscode.MarkdownString();
    body.appendMarkdown(`**${analysis.type}**: ${analysis.message}\n\n`);
    
    if (analysis.suggestion) {
      body.appendCodeblock(analysis.suggestion, 'typescript');
    }
    
    body.appendMarkdown(`\n*Confidence: ${analysis.confidence}*`);
    body.isTrusted = true; // Enable command links
    body.supportHtml = true;
    
    return {
      body,
      mode: vscode.CommentMode.Preview,
      author: { name: '🤖 AI Assistant', iconPath: vscode.Uri.parse('...') },
      contextValue: 'ai-suggestion',
      metadata: {
        confidence: analysis.confidence,
        type: analysis.type,
        reasoning: analysis.reasoning
      }
    };
  }
}
```

### File system provider pattern for synthetic PRs

To enable diff display of synthetic PR content without actual file modifications, implement custom file system providers:

```typescript
class SyntheticPRFileProvider implements vscode.FileSystemProvider {
  private prContent = new Map<string, Uint8Array>();
  
  watch(): vscode.Disposable {
    return new vscode.Disposable(() => {});
  }
  
  stat(uri: vscode.Uri): vscode.FileStat {
    return {
      type: vscode.FileType.File,
      size: this.prContent.get(uri.toString())?.length || 0,
      ctime: Date.now(),
      mtime: Date.now()
    };
  }
  
  readFile(uri: vscode.Uri): Uint8Array {
    const content = this.prContent.get(uri.toString());
    if (!content) {
      throw vscode.FileSystemError.FileNotFound();
    }
    return content;
  }
  
  // Implement other required methods...
}

// Register for synthetic PR schemes
vscode.workspace.registerFileSystemProvider('pr-base', baseProvider);
vscode.workspace.registerFileSystemProvider('pr-head', headProvider);
```

## Trade-offs and architectural decisions

### Native DiffEditor vs Custom Webview

**Chosen: Native DiffEditor**

**Advantages:**
- Zero implementation overhead for diff rendering
- Automatic theme compatibility and accessibility
- Native keyboard shortcuts and navigation
- Consistent performance with large files
- Seamless integration with VSCode search and navigation

**Limitations:**
- Cannot add inline UI elements directly to diff
- Limited control over diff algorithm and display
- No custom highlighting beyond standard diff colors

**Mitigation:** Comment overlays provide interaction points without modifying the diff display itself. The webview panel supplements with rich AI controls where needed.

### CommentController vs Custom Comment System

**Chosen: CommentController API**

**Advantages:**
- Native Comments panel integration without custom UI
- Built-in threading and reply functionality
- Automatic persistence and state management
- Keyboard navigation and accessibility included
- Proven pattern from GitHub PR extension

**Limitations:**
- Fixed comment UI structure
- Limited customization of comment appearance
- Cannot modify core interaction patterns

**Mitigation:** Use `contextValue` property and custom commands to enable AI-specific actions. Supplement with webview panel for advanced AI features.

### Conversation flow implementation

The recommended conversation flow balances automation with human control:

```typescript
class AIConversationManager {
  async initiateReview(files: vscode.Uri[]) {
    // Phase 1: Automatic AI analysis
    const analyses = await this.aiService.analyzeFiles(files);
    
    // Phase 2: Create comment threads for issues
    for (const analysis of analyses) {
      const thread = this.commentController.createCommentThread(
        analysis.uri,
        new vscode.Range(analysis.line, 0, analysis.line, 0),
        [this.createAIComment(analysis)]
      );
      
      thread.canReply = true; // Enable human responses
      thread.contextValue = 'ai-thread-pending';
    }
    
    // Phase 3: Show summary in webview
    this.webviewPanel.show(analyses);
  }
  
  async handleHumanResponse(thread: vscode.CommentThread, response: string) {
    // Add human comment to thread
    const humanComment = this.createHumanComment(response);
    thread.comments = [...thread.comments, humanComment];
    
    // Generate AI refinement based on feedback
    const refinement = await this.aiService.refine(thread, response);
    const aiResponse = this.createAIComment(refinement);
    
    thread.comments = [...thread.comments, aiResponse];
    thread.contextValue = 'ai-thread-active';
  }
}
```

## Performance optimization strategies

### Large diff handling

For repositories with numerous changes, implement progressive loading:

```typescript
class DiffLoader {
  private readonly BATCH_SIZE = 10;
  
  async loadDiffsProgressive(files: FileChange[]) {
    const batches = this.chunk(files, this.BATCH_SIZE);
    
    for (const batch of batches) {
      await Promise.all(batch.map(file => this.showDiff(file)));
      
      // Allow UI to remain responsive
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  
  private chunk<T>(array: T[], size: number): T[][] {
    return array.reduce((chunks, item, index) => {
      const chunkIndex = Math.floor(index / size);
      chunks[chunkIndex] = chunks[chunkIndex] || [];
      chunks[chunkIndex].push(item);
      return chunks;
    }, [] as T[][]);
  }
}
```

### Comment thread optimization

Manage memory and performance with large numbers of comments:

```typescript
class CommentThreadManager {
  private readonly MAX_VISIBLE_THREADS = 50;
  private activeThreads = new Map<string, vscode.CommentThread>();
  
  async updateVisibleThreads(visibleRanges: vscode.Range[]) {
    // Dispose threads outside visible ranges
    for (const [id, thread] of this.activeThreads) {
      if (!this.isThreadVisible(thread, visibleRanges)) {
        thread.dispose();
        this.activeThreads.delete(id);
      }
    }
    
    // Create threads for newly visible ranges
    const threadsToCreate = this.getThreadsInRanges(visibleRanges);
    for (const threadData of threadsToCreate.slice(0, this.MAX_VISIBLE_THREADS)) {
      if (!this.activeThreads.has(threadData.id)) {
        const thread = this.createThread(threadData);
        this.activeThreads.set(threadData.id, thread);
      }
    }
  }
}
```

## Complete implementation roadmap

### Phase 1: Foundation (Week 1-2)
1. Implement CommentController with basic AI comment creation
2. Set up DiffEditor integration with synthetic file system providers
3. Create simple AI service integration for basic code analysis
4. Implement thread creation for AI suggestions

### Phase 2: Conversation System (Week 3-4)
1. Add human reply handling to AI threads
2. Implement AI refinement based on human feedback
3. Create webview panel for AI conversation controls
4. Add context selection for focused analysis

### Phase 3: Enhanced Features (Week 5-6)
1. Implement confidence scoring and visual indicators
2. Add suggestion type categorization (security, performance, style)
3. Create batch operations for multiple suggestions
4. Implement conversation history and persistence

### Phase 4: Polish and Optimization (Week 7-8)
1. Optimize performance for large diffs
2. Add keyboard shortcuts for common actions
3. Implement accessibility features
4. Create comprehensive test suite

## Conclusion

This architecture provides a robust foundation for synthetic PR review with AI conversation capabilities. By leveraging VSCode's native DiffEditor and CommentController APIs while strategically enhancing with custom components, the implementation achieves the optimal balance of native integration, performance, and flexibility for sophisticated AI-human collaboration.

The key to success lies in respecting VSCode's established patterns while thoughtfully extending them for AI-specific requirements. This approach ensures the synthetic PR system feels like a natural extension of VSCode rather than a foreign addition, providing users with a familiar yet powerful code review experience enhanced by AI assistance.