# VSCode Git Extension API capabilities for commit range diffs

## Direct answers to your architecture decision

After extensive research into VSCode's Git Extension API, the evidence strongly suggests a **hybrid approach** for your synthetic PR system: leverage VSCode's Git Extension API for basic operations and UI integration, but implement your own Git operations (potentially with Rust/git2) for complex diff generation and commit range handling. Major production extensions like GitLens and GitHub Pull Requests follow this same pattern due to API limitations.

## Git Extension API diff methods available

The Git Extension API provides limited but useful diff capabilities through a layered architecture. Here's what's actually available:

### Core API access pattern
```typescript
// Standard initialization
const gitExtension = vscode.extensions.getExtension<GitExtension>('vscode.git').exports;
const api = gitExtension.getAPI(1);
const repository = api.repositories[0];

// Available diff-related methods on Repository interface
repository.diff(cached?: boolean): Promise<string>;  // Returns raw diff output
repository.show(ref: string, path: string): Promise<string>;  // File content at ref
repository.getCommit(ref: string): Promise<Commit>;
repository.getMergeBase(ref1: string, ref2: string): Promise<string>;
repository.status(): Promise<Change[]>;
```

### URI-based diff mechanism
VSCode uses URI transformation for accessing different file versions:

```typescript
// Built-in function from Git Extension
export function toGitUri(uri: Uri, ref: string): Uri {
    return uri.with({
        scheme: 'git',
        path: uri.path,
        query: JSON.stringify({
            path: uri.fsPath,
            ref: ref  // Can be HEAD, HEAD~2, commit SHA, branch name
        })
    });
}

// Open diff viewer programmatically
await vscode.commands.executeCommand('vscode.diff',
    toGitUri(fileUri, 'HEAD~2'),  // Original version
    fileUri,                       // Current version
    'Review changes'               // Title
);
```

**Critical limitation**: The API **lacks a direct `diffBetween(commit1, commit2)` method**. The `repository.diff()` method only returns raw git diff output for working tree changes, not arbitrary commit ranges.

## Commit range handling capabilities

VSCode's Git Extension API has **limited native support** for commit range operations:

### What works
- **Commit reference resolution**: The API accepts standard Git references (HEAD~2, branch names, commit SHAs) in the `toGitUri()` function
- **Single commit retrieval**: `repository.getCommit(ref)` resolves individual commits
- **Merge base detection**: `repository.getMergeBase()` finds common ancestors

### What doesn't work
- **No built-in commit range parsing**: Extensions must parse ranges like `abc123..def456` themselves
- **No bulk diff operations**: Cannot efficiently get diffs for multiple commits at once
- **Limited history traversal**: No native support for complex range expressions

### Practical workaround from production extensions
```typescript
// GitHub PR extension approach
async function getCommitRangeDiff(baseRef: string, headRef: string) {
    // Get individual commits
    const baseCommit = await repository.getCommit(baseRef);
    const headCommit = await repository.getCommit(headRef);
    
    // For actual diff, extensions shell out to git
    const gitPath = await api.git.path;
    const result = await exec(gitPath, 
        ['diff', '--name-status', baseRef, headRef],
        { cwd: repository.rootUri.fsPath }
    );
    
    return parseGitDiffOutput(result.stdout);
}
```

## File change detection without parsing raw output

Unfortunately, the Git Extension API **does not provide** structured file change data. There's no method that returns file-level statistics (+15 -3) or structured change lists for commit ranges. 

### Current reality
Extensions must either:
1. Parse raw git diff output from `repository.diff()`
2. Shell out to git commands directly
3. Use the npm `simple-git` package (most common approach)

### Best available approach using VSCode APIs
```typescript
// Using Source Control API for working tree changes
const changes = await repository.status();
// Returns: Change[] with resourceUri and status (modified/added/deleted)

// For commit ranges, must combine multiple approaches
async function getFileChangesForRange(baseRef: string, headRef: string) {
    // No direct API - must use workarounds
    const baseFiles = new Set<string>();
    const headFiles = new Set<string>();
    
    // Option 1: Shell out (what most extensions do)
    const diffResult = await execGit(['diff', '--name-status', baseRef, headRef]);
    
    // Option 2: Use simple-git npm package
    const git = simpleGit(repository.rootUri.fsPath);
    const diff = await git.diffSummary([baseRef, headRef]);
    
    return diff.files; // Structured data with insertions/deletions
}
```

## Diff viewer integration patterns

VSCode provides good support for displaying diffs, but limited support for PR-style multi-file reviews:

### Single file diff display
```typescript
// Standard approach used by all PR extensions
async function showFileDiff(file: ChangedFile, baseRef: string, headRef: string) {
    const leftUri = toGitUri(file.uri, baseRef);
    const rightUri = toGitUri(file.uri, headRef);
    
    await vscode.commands.executeCommand('vscode.diff',
        leftUri,
        rightUri,
        `${file.filename} (${baseRef}...${headRef})`
    );
}
```

### Multi-file PR review interface
VSCode **lacks native support** for showing multiple file diffs simultaneously. Extensions implement custom solutions:

```typescript
// GitHub PR extension pattern
class PRTreeDataProvider implements vscode.TreeDataProvider<FileNode> {
    getChildren(element?: FileNode): FileNode[] {
        if (!element) {
            return this.changedFiles; // List all changed files
        }
        return []; // Files are leaf nodes
    }
    
    getTreeItem(element: FileNode): vscode.TreeItem {
        return {
            label: element.filename,
            command: {
                command: 'vscode.diff',
                arguments: [element.originalUri, element.modifiedUri, element.label]
            }
        };
    }
}

// Register custom view
vscode.window.createTreeView('prReview', {
    treeDataProvider: new PRTreeDataProvider()
});
```

### Comment Controller integration
```typescript
// For PR-style commenting on diffs
const commentController = vscode.comments.createCommentController(
    'synthetic-pr',
    'Synthetic PR Review'
);

// Create threads on specific lines
const thread = commentController.createCommentThread(
    fileUri,
    new vscode.Range(10, 0, 10, 0),
    []  // Initial comments
);
thread.canReply = true;
thread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded;
```

## Source Control API relationship and QuickDiffProvider

The architecture involves three interconnected systems:

### Layer hierarchy
1. **Source Control API** (`vscode.scm`) - Generic framework for any SCM
2. **Git Extension** - Specific implementation using Source Control API
3. **QuickDiffProvider** - Bridges SCM and editor diff features

### QuickDiffProvider for gutter indicators
```typescript
class CustomQuickDiffProvider implements vscode.QuickDiffProvider {
    provideOriginalResource(uri: vscode.Uri): vscode.Uri | undefined {
        // Return URI of comparison base
        return toGitUri(uri, 'HEAD~2'); // Compare with 2 commits ago
    }
}

// Register with source control
const scm = vscode.scm.createSourceControl('synthetic-pr', 'Synthetic PR');
scm.quickDiffProvider = new CustomQuickDiffProvider();
```

This provides the green/red indicators in the editor gutter but **doesn't help with commit range diffs**.

## Performance considerations and caching strategies

VSCode's Git Extension API has significant performance limitations for large repositories and commit ranges:

### Built-in caching
- **Limited caching**: VSCode caches repository state but not diff results
- **No diff result caching**: Each `repository.diff()` call re-executes git
- **File content caching**: Virtual file system providers must implement their own caching

### Production extension patterns
```typescript
// Cache implementation from GitHub PR extension
class DiffCache {
    private cache = new Map<string, CachedDiff>();
    
    async getDiff(baseRef: string, headRef: string): Promise<DiffResult> {
        const key = `${baseRef}:${headRef}`;
        
        if (this.cache.has(key)) {
            const cached = this.cache.get(key);
            if (Date.now() - cached.timestamp < 5 * 60 * 1000) { // 5 min TTL
                return cached.data;
            }
        }
        
        const diff = await this.computeDiff(baseRef, headRef);
        this.cache.set(key, { data: diff, timestamp: Date.now() });
        return diff;
    }
    
    private async computeDiff(baseRef: string, headRef: string): Promise<DiffResult> {
        // Expensive operation - shell out to git
        const result = await execGit(['diff', '--numstat', baseRef, headRef]);
        return parseDiffResult(result);
    }
}
```

### Performance best practices
- **Lazy loading**: Only compute diffs when files are actually viewed
- **Incremental updates**: Use file watchers for working tree changes
- **Background processing**: Compute large diffs in background tasks
- **Pagination**: Load changed files in batches for large PRs

## Practical implementation recommendation

Based on the research, here's the recommended architecture for your synthetic PR system:

### Hybrid architecture approach
```typescript
// 1. Use Git Extension API for basic operations
const gitApi = vscode.extensions.getExtension('vscode.git').exports.getAPI(1);
const repository = gitApi.repositories[0];

// 2. Implement custom diff operations for complex scenarios
class SyntheticPRDiffProvider {
    private gitPath: string;
    private cache = new DiffCache();
    
    async getCommitRangeDiff(range: string): Promise<FileDiff[]> {
        // Parse range (HEAD~2, abc123..def456, etc.)
        const [baseRef, headRef] = this.parseRange(range);
        
        // Try VSCode API first for simple cases
        if (range === 'HEAD') {
            const changes = await repository.status();
            return this.convertToFileDiffs(changes);
        }
        
        // Fall back to git2/shell for complex ranges
        return this.cache.getDiff(baseRef, headRef);
    }
    
    private async computeDiffWithGit2(baseRef: string, headRef: string) {
        // Use Rust git2 bindings or shell out
        // This gives you full control over diff generation
    }
}

// 3. Integrate with VSCode UI
class PRReviewProvider {
    async showDiff(file: FileDiff) {
        const leftUri = toGitUri(file.uri, file.baseRef);
        const rightUri = toGitUri(file.uri, file.headRef);
        
        await vscode.commands.executeCommand('vscode.diff',
            leftUri, rightUri, file.label
        );
    }
    
    setupCommentController() {
        const controller = vscode.comments.createCommentController(
            'synthetic-pr', 'LLM Review'
        );
        // Configure for PR-style reviews
    }
}
```

### Decision matrix: VSCode API vs Rust git2

**Use VSCode Git Extension API for:**
- Repository discovery and basic status
- Integration with VSCode's SCM UI
- Simple file content retrieval
- Opening diff editors
- Comment Controller setup

**Implement with Rust git2 for:**
- Complex commit range parsing and diff generation
- Bulk file change detection with statistics
- Performance-critical diff operations
- Custom diff algorithms or filtering
- Advanced Git operations not exposed by the API

### Fallback implementation pattern
```typescript
class GitOperations {
    async getDiff(baseRef: string, headRef: string): Promise<DiffResult> {
        try {
            // Try VSCode API first
            if (this.isSimpleCase(baseRef, headRef)) {
                return await this.useVSCodeAPI(baseRef, headRef);
            }
        } catch (e) {
            // API limitations or errors
        }
        
        // Fallback to custom implementation
        if (this.hasRustBindings()) {
            return await this.useGit2Rust(baseRef, headRef);
        }
        
        // Final fallback to shell
        return await this.shellOutToGit(baseRef, headRef);
    }
}
```

## Conclusion

VSCode's Git Extension API provides useful building blocks but **lacks the comprehensive diff operations needed for a synthetic PR system**. The API doesn't offer methods for generating structured diffs between arbitrary commit ranges, forcing extensions to implement their own Git operations. This is why major extensions like GitLens, GitHub Pull Requests, and GitLab Workflow all use a hybrid approach combining VSCode APIs with custom Git implementations.

For your synthetic PR system, leverage VSCode's infrastructure for UI integration and basic operations, but implement core diff generation using Rust git2 bindings. This gives you the performance and control needed while maintaining compatibility with VSCode's excellent diff viewing and commenting infrastructure.