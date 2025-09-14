import * as vscode from 'vscode';
import { SyntheticPRTreeProvider } from './syntheticPRTreeProvider';

interface SyntheticPRData {
    review_id: string;
    title: string;
    description: any;
    commit_range: string;
    files_changed: FileChange[];
    comment_threads: CommentThread[];
    status: string;
}

interface FileChange {
    path: string;
    status: string;
    additions: number;
    deletions: number;
    hunks: DiffHunk[];
}

interface DiffHunk {
    old_start: number;
    old_lines: number;
    new_start: number;
    new_lines: number;
    lines: DiffLine[];
}

interface DiffLine {
    line_type: 'Context' | 'Added' | 'Removed';
    old_line_number?: number;
    new_line_number?: number;
    content: string;
}

interface CommentThread {
    id: string;
    file_path: string;
    line_number: number;
    comment_type: 'insight' | 'question' | 'todo' | 'fixme';
    content: string;
}

/**
 * Content provider for synthetic diff content
 */
class DialecticDiffContentProvider implements vscode.TextDocumentContentProvider {
    private contentMap = new Map<string, string>();

    setContent(uri: vscode.Uri, content: string): void {
        this.contentMap.set(uri.toString(), content);
    }

    provideTextDocumentContent(uri: vscode.Uri): string {
        return this.contentMap.get(uri.toString()) || '';
    }
}

/**
 * Manages synthetic pull request UI components
 * 
 * Creates unified PR interface using TreeDataProvider for navigation
 * and CommentController for in-line code comments.
 */
export class SyntheticPRProvider implements vscode.Disposable {
    private commentController: vscode.CommentController;
    private treeProvider: SyntheticPRTreeProvider;
    private diffContentProvider: DialecticDiffContentProvider;
    private currentPR: SyntheticPRData | null = null;
    private onCommentCallback?: (comment: string, filePath: string, lineNumber: number) => void;

    constructor(private context: vscode.ExtensionContext) {
        // Create diff content provider for virtual diff content
        this.diffContentProvider = new DialecticDiffContentProvider();
        context.subscriptions.push(
            vscode.workspace.registerTextDocumentContentProvider('symposium-diff', this.diffContentProvider)
        );

        // Create comment controller for in-line comments
        this.commentController = vscode.comments.createCommentController(
            'symposium-synthetic-pr',
            'Synthetic PR Comments'
        );
        
        // Configure comment controller options
        this.commentController.options = {
            prompt: 'Add a comment...',
            placeHolder: 'Type your comment here'
        };
        
        this.commentController.commentingRangeProvider = {
            provideCommentingRanges: (document: vscode.TextDocument) => {
                console.log(`[COMMENTING] provideCommentingRanges called for: ${document.uri.toString()}`);
                
                // Allow commenting on any line in files that are part of the current PR
                if (!this.currentPR) {
                    console.log(`[COMMENTING] No current PR - returning empty ranges`);
                    return [];
                }
                
                // Handle comment input URIs (comment://) - always allow commenting
                if (document.uri.scheme === 'comment') {
                    console.log(`[COMMENTING] Comment input URI - allowing full document range`);
                    const lineCount = document.lineCount;
                    const range = new vscode.Range(0, 0, Math.max(0, lineCount - 1), 0);
                    return [range];
                }
                
                // Handle regular file URIs
                const filePath = vscode.workspace.asRelativePath(document.uri);
                const isInPR = this.currentPR.files_changed.some(f => f.path === filePath);
                
                console.log(`[COMMENTING] File ${filePath} in PR: ${isInPR}`);
                
                if (isInPR) {
                    // Return a single range covering the entire document for multi-line support
                    const lineCount = document.lineCount;
                    const range = new vscode.Range(0, 0, Math.max(0, lineCount - 1), 0);
                    console.log(`[COMMENTING] Returning range for ${lineCount} lines: ${range.start.line}-${range.end.line}`);
                    return [range];
                }
                
                console.log(`[COMMENTING] File not in PR - returning empty ranges`);
                return [];
            }
        };

        // Handle new comment creation when user clicks "+" icon
        this.commentController.options = {
            prompt: 'Add a comment...',
            placeHolder: 'Type your comment here'
        };

        // Handle comment thread creation and replies
        this.setupCommentHandlers();

        // Create tree provider for PR navigation
        console.log('[SYNTHETIC PR] Creating tree provider');
        this.treeProvider = new SyntheticPRTreeProvider();
        
        // Register tree view
        console.log('[SYNTHETIC PR] Registering tree view with ID: socratic-shell.syntheticPR');
        const treeView = vscode.window.createTreeView('socratic-shell.syntheticPR', {
            treeDataProvider: this.treeProvider
        });
        console.log('[SYNTHETIC PR] Tree view created successfully:', !!treeView);

        // Register diff command
        const diffCommand = vscode.commands.registerCommand('socratic-shell.showFileDiff', 
            (filePath: string) => this.showFileDiff(filePath)
        );

        // Register comment reply command
        const commentReplyCommand = vscode.commands.registerCommand('socratic-shell.addCommentReply',
            (thread: vscode.CommentThread, text: string) => this.addCommentReply(thread, text)
        );

        // Register add comment command (for new comments)
        const addCommentCommand = vscode.commands.registerCommand('socratic-shell.addComment',
            (reply: vscode.CommentReply) => this.handleCommentSubmission(reply)
        );

        // Register toggle comments command
        const toggleCommentsCommand = vscode.commands.registerCommand('socratic-shell.toggleComments',
            () => this.treeProvider.toggleCommentsExpansion()
        );

        context.subscriptions.push(this.commentController, treeView, diffCommand, commentReplyCommand, addCommentCommand, toggleCommentsCommand);
    }

    /**
     * Create a new synthetic PR from MCP server data
     */
    async createSyntheticPR(prData: SyntheticPRData): Promise<void> {
        const startTime = Date.now();
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: createSyntheticPR called with: ${prData.title}`);
        this.currentPR = prData;
        
        // Update tree view
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Calling treeProvider.updatePR`);
        this.treeProvider.updatePR(prData);
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: treeProvider.updatePR completed`);
        
        // Create comment threads for each AI insight
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Creating ${prData.comment_threads.length} comment threads`);
        for (const thread of prData.comment_threads) {
            await this.createCommentThread(thread);
        }
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: All comment threads created`);

        // Show status message
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Showing status message`);
        vscode.window.showInformationMessage(
            `Synthetic PR created: ${prData.title} (${prData.files_changed.length} files changed)`
        );
        console.log(`[SYNTHETIC PR] ${Date.now() - startTime}ms: createSyntheticPR completed`);
    }

    /**
     * Update existing synthetic PR
     */
    async updateSyntheticPR(prData: SyntheticPRData): Promise<void> {
        if (!this.currentPR || this.currentPR.review_id !== prData.review_id) {
            // If no current PR or different PR, treat as create
            return this.createSyntheticPR(prData);
        }

        this.currentPR = prData;
        
        // Update tree view
        this.treeProvider.updatePR(prData);
        
        // Recreate comment threads
        this.commentController.dispose();
        this.commentController = vscode.comments.createCommentController(
            'symposium-synthetic-pr',
            `PR: ${prData.title}`
        );
        
        for (const thread of prData.comment_threads) {
            await this.createCommentThread(thread);
        }
    }

    /**
     * Show GitHub-style diff for a file
     */
    private async showFileDiff(filePath: string): Promise<void> {
        console.log(`[DIFF] Starting showFileDiff for: ${filePath}`);
        
        if (!this.currentPR) {
            console.log('[DIFF] ERROR: No active synthetic PR');
            vscode.window.showErrorMessage('No active synthetic PR');
            return;
        }

        const fileChange = this.currentPR.files_changed.find(f => f.path === filePath);
        if (!fileChange) {
            console.log(`[DIFF] ERROR: File not found in PR: ${filePath}`);
            vscode.window.showErrorMessage(`File not found in PR: ${filePath}`);
            return;
        }
        
        console.log(`[DIFF] Found file change: ${fileChange.status}, ${fileChange.additions}+/${fileChange.deletions}-, ${fileChange.hunks.length} hunks`);

        try {
            // Resolve relative path to absolute path
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }
            
            const absolutePath = vscode.Uri.joinPath(workspaceFolder.uri, filePath);
            console.log(`[DIFF] Resolved absolute path: ${absolutePath.toString()}`);
            
            // Get "after" content from current file
            const currentDocument = await vscode.workspace.openTextDocument(absolutePath);
            const modifiedContent = currentDocument.getText();
            console.log(`[DIFF] Current file content length: ${modifiedContent.length} chars`);

            // Generate "before" content by reverse-applying hunks
            const originalContent = await this.generateOriginalContent(fileChange, modifiedContent);
            console.log(`[DIFF] Generated original content length: ${originalContent.length} chars`);

            // Create URIs for diff content provider
            const originalUri = vscode.Uri.parse(`symposium-diff:${filePath}?original`);
            const modifiedUri = absolutePath; // Use actual file for "after" state
            console.log(`[DIFF] Original URI: ${originalUri.toString()}`);
            console.log(`[DIFF] Modified URI: ${modifiedUri.toString()}`);

            // Store original content in provider
            this.diffContentProvider.setContent(originalUri, originalContent);
            console.log('[DIFF] Stored original content in provider');

            // Show diff using VSCode's native diff viewer with automatic highlighting
            console.log('[DIFF] Calling vscode.diff command...');
            await vscode.commands.executeCommand('vscode.diff', 
                originalUri, 
                modifiedUri, 
                `${filePath} (PR Diff)`
            );
            console.log('[DIFF] vscode.diff command completed successfully');

        } catch (error) {
            console.error('[DIFF] Failed to show file diff:', error);
            vscode.window.showErrorMessage(`Failed to show diff for ${filePath}`);
        }
    }

    /**
     * Generate original file content by reverse-applying hunks
     */
    private async generateOriginalContent(fileChange: FileChange, currentContent: string): Promise<string> {
        console.log(`[HUNK] Starting generateOriginalContent for ${fileChange.path}`);
        console.log(`[HUNK] Processing ${fileChange.hunks.length} hunks`);
        
        try {
            const currentLines = currentContent.split('\n');
            const originalLines = [...currentLines];
            console.log(`[HUNK] Current file has ${currentLines.length} lines`);
            
            // Sort hunks by line number (descending) to apply in reverse order
            const sortedHunks = [...fileChange.hunks].sort((a, b) => b.new_start - a.new_start);
            console.log(`[HUNK] Sorted hunks by new_start (desc): ${sortedHunks.map(h => h.new_start).join(', ')}`);
            
            for (const hunk of sortedHunks) {
                console.log(`[HUNK] Processing hunk at line ${hunk.new_start} with ${hunk.lines.length} lines`);
                
                // Process lines in reverse order within each hunk
                const hunkLines = [...hunk.lines].reverse();
                let lineOffset = hunk.new_lines - 1;
                
                for (const line of hunkLines) {
                    const targetLine = hunk.new_start - 1 + lineOffset;
                    console.log(`[HUNK] Processing ${line.line_type} line at target ${targetLine}: "${line.content.substring(0, 50)}..."`);
                    
                    if (line.line_type === 'Added') {
                        // Remove added lines from original
                        if (targetLine >= 0 && targetLine < originalLines.length) {
                            console.log(`[HUNK] Removing added line at ${targetLine}`);
                            originalLines.splice(targetLine, 1);
                        }
                        lineOffset--;
                    } else if (line.line_type === 'Removed') {
                        // Restore deleted lines to original
                        const content = line.content.startsWith('-') ? line.content.substring(1) : line.content;
                        console.log(`[HUNK] Restoring removed line at ${targetLine + 1}: "${content.substring(0, 50)}..."`);
                        originalLines.splice(targetLine + 1, 0, content);
                    } else if (line.line_type === 'Context') {
                        // Context lines stay the same
                        lineOffset--;
                    }
                }
            }
            
            console.log(`[HUNK] Generated original content with ${originalLines.length} lines`);
            return originalLines.join('\n');
            
        } catch (error) {
            console.error('[HUNK] Failed to generate original content:', error);
            // Fallback to empty content for minimal diff display
            return '';
        }
    }

    /**
     * Setup handlers for comment creation and replies
     */
    private setupCommentHandlers(): void {
        // Set comment controller options to enable submit button
        this.commentController.options = {
            prompt: 'Add a comment...',
            placeHolder: 'Type your comment here'
        };

        // The key insight from the documentation: VSCode automatically creates comment threads
        // when users click the "+" icon, but we need to ensure they have canReply = true.
        // This happens in our commentingRangeProvider and when we create threads manually.
        // The submit button appears when both options are set AND canReply = true on the thread.
    }

    /**
     * Create a comment thread for an AI insight on diff view
     */
    private async createCommentThread(thread: CommentThread): Promise<void> {
        try {
            // Use regular file URI for comments (they'll appear on both diff and normal views)
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            if (!workspaceFolder) {
                return;
            }
            const uri = vscode.Uri.joinPath(workspaceFolder.uri, thread.file_path);
            
            // Create a simple range for the comment
            const range = new vscode.Range(
                Math.max(0, thread.line_number - 1), 0,
                Math.max(0, thread.line_number - 1), 0
            );
            
            const commentThread = this.commentController.createCommentThread(uri, range, []);
            
            // Make comments expanded by default and allow replies
            commentThread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded;
            commentThread.canReply = true;

            // Create comment with AI insight
            const comment: vscode.Comment = {
                body: new vscode.MarkdownString(this.formatComment(thread)),
                mode: vscode.CommentMode.Preview,
                author: {
                    name: 'AI Assistant'
                }
            };
            
            commentThread.comments = [comment];
            commentThread.label = `${this.getCommentIcon(thread.comment_type)} ${thread.comment_type.toUpperCase()}`;
            
        } catch (error) {
            console.error(`Failed to create comment thread for ${thread.file_path}:${thread.line_number}`, error);
        }
    }

    /**
     * Format comment content with type-specific styling
     */
    private formatComment(thread: CommentThread): string {
        const icon = this.getCommentIcon(thread.comment_type);
        const typeLabel = thread.comment_type.toUpperCase();
        
        return `${icon} **${typeLabel}**\n\n${thread.content}`;
    }

    /**
     * Get icon for comment type
     */
    private getCommentIcon(type: string): string {
        switch (type) {
            case 'insight': return '💡';
            case 'question': return '❓';
            case 'todo': return '📝';
            case 'fixme': return '🔧';
            default: return '💬';
        }
    }

    /**
     * Handle adding a reply to a comment thread
     */
    private addCommentReply(thread: vscode.CommentThread, text: string): void {
        const newComment: vscode.Comment = {
            body: new vscode.MarkdownString(text),
            mode: vscode.CommentMode.Preview,
            author: {
                name: vscode.env.uriScheme === 'vscode' ? 'User' : 'Developer'
            }
        };

        thread.comments = [...thread.comments, newComment];
    }

    /**
     * Handle comment submission from VSCode (both new comments and replies)
     */
    private handleCommentSubmission(reply: vscode.CommentReply): void {
        const newComment: vscode.Comment = {
            body: new vscode.MarkdownString(reply.text),
            mode: vscode.CommentMode.Preview,
            author: {
                name: vscode.env.uriScheme === 'vscode' ? 'User' : 'Developer'
            }
        };

        reply.thread.comments = [...reply.thread.comments, newComment];
        
        // Ensure the thread can accept more replies
        reply.thread.canReply = true;

        // Send comment as feedback to LLM
        if (this.onCommentCallback && reply.thread.range) {
            const uri = reply.thread.uri;
            const lineNumber = reply.thread.range.start.line + 1; // Convert to 1-based
            const filePath = uri.scheme === 'symposium-diff' ? 
                uri.path.replace('/diff/', '') : // Extract file path from diff URI
                vscode.workspace.asRelativePath(uri);
            
            this.onCommentCallback(reply.text, filePath, lineNumber);
        }
    }

    /**
     * Set callback for when user submits a comment
     */
    setCommentCallback(callback: (comment: string, filePath: string, lineNumber: number) => void): void {
        this.onCommentCallback = callback;
    }

    /**
     * Clear the current PR from tree view
     */
    clearPR(): void {
        this.treeProvider.clearPR();
        this.currentPR = null;
    }

    dispose(): void {
        this.commentController.dispose();
        this.treeProvider.clearPR();
    }
}
