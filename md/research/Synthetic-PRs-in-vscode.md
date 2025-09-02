# Building synthetic pull requests in VS Code

VS Code's Pull Request functionality leverages the **CommentController API** rather than a dedicated PR Provider interface, enabling extensions to create rich code review experiences with in-editor commenting, diff viewing, and collaborative workflows. This architecture allows for building synthetic PR systems that work entirely with local data, though it requires custom implementation beyond the standard remote-focused extensions.

The core finding is that **local/synthetic PRs are technically feasible** without requiring real Git remotes. VS Code provides robust APIs for local Git operations through its Git Extension API, allowing access to repository state, diff generation, and file changes. Extensions can create PR-like workflows using local commits, staged changes, or even working directory modifications without ever pushing to a remote repository. This makes it ideal for reviewing AI-generated code locally before committing or pushing changes.

## Core API architecture reveals flexible foundation

The Pull Request Provider functionality in VS Code has evolved from a provider-based pattern to the current **CommentController-based architecture**. This shift provides greater flexibility for extensions to implement custom review workflows.

### Essential TypeScript interfaces power the system

The **CommentController** serves as the central management entity, created through `vscode.comments.createCommentController()`. This controller manages comment threads, handles user interactions, and provides the foundation for PR-like functionality. Each controller requires a unique identifier and label, and can optionally specify where comments are allowed through a `CommentingRangeProvider`.

The **CommentThread** interface represents a collection of comments at a specific location in code. Threads attach to precise locations using `vscode.Range` objects and maintain properties like `canReply`, `collapsibleState`, and `contextValue` for conditional UI elements. The thread's `comments` array contains individual **Comment** objects, each with an author, body (supporting Markdown), mode (editing or preview), and optional reactions.

For registration, extensions activate through standard VS Code patterns, typically using activation events like `onStartupFinished` or `workspaceContains:**/.git`. The package.json contribution points define commands, menus, and configuration settings specific to PR functionality. Menu contributions use `when` clauses to show contextual actions based on comment thread state or provider type.

### Command and event architecture enables rich interactions

Extensions register command handlers for user actions like approving changes, requesting reviews, or adding comments. The contribution system allows contextual menu items in comment threads, SCM views, and editor contexts. Event handlers process user interactions through callbacks, with the `reactionHandler` managing comment reactions and command implementations handling approval workflows.

## Local data capabilities unlock synthetic PR workflows

VS Code's architecture **does not require real Git remotes** for PR-like functionality. The Git Extension API provides complete access to local repository state, including `diffWithHEAD()` for uncommitted changes, `diffBetween()` for commit ranges, and full branch management capabilities.

### Git Extension API enables local diff generation

The Git Extension API, accessed through `vscode.extensions.getExtension('vscode.git')`, provides comprehensive local repository access. Extensions can retrieve staged versus unstaged changes, generate diffs between any commits or branches, and access the complete commit history. This enables creating PRs from local branches without remote counterparts, working directory changes, or even hypothetical file modifications.

Creating synthetic diffs leverages the `QuickDiffProvider` interface, which allows custom comparison logic through `provideOriginalResource()`. This method can return any URI representing the "before" state, whether from a specific commit, the staging area, or synthetic content. The Source Control API's `createSourceControl()` method enables custom SCM providers that work entirely with local data.

### Implementation strategy for local PR systems

The recommended approach combines multiple VS Code APIs to create a cohesive local PR experience. Create a custom Source Control Provider for managing synthetic PRs, use the Git Extension API for repository operations, implement TreeDataProvider for PR visualization, leverage QuickDiffProvider for diff views, and store metadata locally using workspace state or JSON files.

A local PR data structure might include fields for ID, title, source/target branches, file changes, commits, and status. Since VS Code's existing PR extensions expect remote repositories, workarounds include creating mock remote URLs for compatibility, implementing synthetic branch references, and handling merge operations locally through Git commands.

## Comment system integration enables code review workflows

The comment system's precise line targeting makes it ideal for code review scenarios. Comments attach to specific ranges using `vscode.Range`, supporting single-line comments, multi-line selections, or file-level discussions.

### Pre-populating comments streamlines review process

When creating synthetic PRs, extensions can pre-populate comment threads programmatically. This is particularly useful for AI-generated code reviews, where the system might automatically identify potential issues or suggest improvements. The `createCommentThread()` method accepts an initial array of comments, allowing immediate population of review feedback.

Thread management includes dynamic range updates as code changes, collapsible state control for UI organization, and context values for enabling specific menu items. The comment thread lifecycle supports creation, modification, and deletion through programmatic APIs, with proper event firing to update the UI.

### Threaded discussions mirror traditional PR interfaces

Each comment thread maintains its own discussion context, with `canReply` controlling whether users can respond. The reply mechanism uses command handlers that receive `CommentReply` objects containing the thread reference and text. This enables building conversation flows similar to GitHub or GitLab, entirely within the local VS Code environment.

## File diff integration completes the review experience

VS Code's diff viewer integration relies on custom content providers that supply file versions for comparison. The pattern involves registering a `TextDocumentContentProvider` with a custom URI scheme, then using `vscode.diff` command to open side-by-side comparisons.

### Custom URI schemes enable flexible diff scenarios

A PR diff provider might use URIs like `pr-diff://pr/123/src/file.ts?version=base` to identify specific file versions. The provider's `provideTextDocumentContent()` method parses these URIs and returns appropriate content, whether from Git history, working directory, or synthetic modifications.

Integration with SCM Quick Diff provides inline change indicators in the editor gutter. By implementing `quickDiffProvider` on a Source Control instance, extensions can show changes relative to any baseline, not just the last commit. This is particularly powerful for synthetic PRs that might compare against hypothetical states.

## State persistence ensures continuity across sessions

PR providers use multiple storage mechanisms to maintain state across VS Code sessions. The **Memento API** provides key-value storage at both global (`context.globalState`) and workspace (`context.workspaceState`) levels. Global state suits user preferences and authentication tokens, while workspace state handles project-specific PR data.

### Production implementations reveal proven patterns

The Azure DevOps PR extension, a fork of GitHub's implementation, demonstrates effective state management using both configuration files and Memento storage. It stores authentication tokens in secure storage via `context.secrets`, caches PR data with timestamps for expiration, and persists user preferences in workspace settings.

GitLab's extension shows another pattern, using file system storage through `context.globalStorageUri` for larger datasets. This approach works well for caching diff content or storing extensive PR metadata that might exceed Memento storage limits.

### Synthetic PR persistence strategies

For local/synthetic PRs, a hybrid approach works best. Store PR metadata in workspace state for quick access, save detailed content in JSON files within the workspace, use global state for user preferences affecting all projects, and implement proper cleanup in the extension's `deactivate()` function.

## User interaction patterns mirror remote PR workflows

VS Code's contribution system enables rich user interactions through commands, menus, and status bar items. The command palette, context menus, and inline actions provide multiple entry points for PR operations.

### Approval and review actions integrate naturally

Command registration for approval workflows follows standard VS Code patterns. Commands receive comment thread contexts, allowing targeted actions like approving specific file changes. The when clause system enables showing actions conditionally based on PR state, user permissions, or file types.

Status bar integration provides at-a-glance PR information, showing counts of open reviews or pending actions. Tree view providers organize PRs hierarchically, with collapsible sections for different states or categories. WebView providers enable rich HTML interfaces for complex review forms or visualization.

### Event-driven architecture ensures responsive UI

The event system uses VS Code's EventEmitter pattern for state changes. PR providers fire events when data updates, triggering UI refreshes across all views. This reactive pattern ensures consistency between tree views, editor decorations, and status indicators.

## Implementation examples guide practical development

Several production extensions demonstrate these concepts effectively. The **GitHub Pull Requests and Issues** extension, while remote-focused, provides the most comprehensive implementation reference. Its source code reveals patterns for comment management, diff generation, and state synchronization.

### Azure DevOps extension offers accessible architecture

The Azure DevOps PR extension, being a GitHub extension fork, maintains similar architecture while adapting to different API requirements. Its codebase demonstrates authentication handling, comment thread creation, and review workflow implementation. The extension's approach to offline scenarios provides insights for synthetic PR handling.

### VS Code samples repository provides foundation

Microsoft's extension samples repository includes a source-control-sample that demonstrates custom SCM provider implementation without remote repositories. While not PR-specific, it shows local state management, resource decoration, and commit operations that form the foundation for synthetic PR systems.

## Architectural recommendations for synthetic PR systems

Building a robust synthetic PR system requires careful architectural decisions. Start with a clear separation between data models and UI components. Implement a service layer abstracting Git operations from PR logic. Use dependency injection for testability and flexibility. Create interfaces matching existing PR provider patterns for potential future compatibility.

The recommended architecture includes a **PRService** managing PR lifecycle and state, **GitService** wrapping Git Extension API calls, **CommentService** handling comment thread operations, **DiffService** generating and caching file comparisons, and **StorageService** abstracting persistence mechanisms.

Performance optimization becomes critical with local data processing. Implement lazy loading for PR content, cache generated diffs aggressively, debounce UI updates during rapid changes, and use virtual scrolling for large PR lists. Background processing for diff generation prevents UI blocking during complex operations.

## Conclusion

VS Code's flexible API architecture enables building sophisticated synthetic PR systems that operate entirely with local data. While no dedicated PR Provider interface exists, the combination of CommentController API, Git Extension API, and Source Control API provides all necessary building blocks. The absence of remote repository requirements makes this approach ideal for AI code review scenarios, allowing immediate feedback on generated code before any commits or remote interactions.

Key implementation considerations include leveraging the CommentController for review UI, using Git Extension API for local repository access, implementing custom storage for PR metadata, creating TreeDataProvider for PR organization, and following event-driven patterns for responsive interfaces. Production extensions from GitHub, GitLab, and Azure DevOps provide proven patterns, though adaptation for synthetic scenarios requires creative approaches to state management and diff generation.

The path forward involves building a custom extension that combines these APIs into a cohesive local PR experience, enabling familiar code review workflows for AI-generated code without external dependencies.