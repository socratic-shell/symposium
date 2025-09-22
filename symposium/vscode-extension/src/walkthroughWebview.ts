import * as vscode from 'vscode';
import * as crypto from 'crypto';
import * as path from 'path';
import * as MarkdownIt from 'markdown-it';
import { openSymposiumUrl } from './fileNavigation';
import { Bus } from './bus';

// Placement state for unified link and comment management
interface PlacementState {
    isPlaced: boolean;
    chosenLocation: any; // FileRange, SearchResult, or other location type
    wasAmbiguous: boolean; // Whether this item had multiple possible locations
}

// Reuse types from synthetic PR system
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

/**
 * Content provider for walkthrough diff content
 */
class WalkthroughDiffContentProvider implements vscode.TextDocumentContentProvider {
    private contentMap = new Map<string, string>();

    setContent(uri: vscode.Uri, content: string): void {
        this.contentMap.set(uri.toString(), content);
    }

    provideTextDocumentContent(uri: vscode.Uri): string | undefined {
        return this.contentMap.get(uri.toString());
    }
}

type WalkthroughElement =
    | string  // ResolvedMarkdownElement (now serialized as plain string)
    | { comment: any }  // Simplified for now
    | { files: FileChange[] }  // GitDiffElement - named field serializes as {"files": [...]}
    | { action: { button: string; description?: string; tell_agent?: string } };

interface WalkthroughData {
    introduction?: WalkthroughElement[];
    highlights?: WalkthroughElement[];
    changes?: WalkthroughElement[];
    actions?: WalkthroughElement[];
}

export class WalkthroughWebviewProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'symposium.walkthrough';

    private _view?: vscode.WebviewView;
    private md: MarkdownIt;
    private baseUri?: vscode.Uri;
    private diffContentProvider: WalkthroughDiffContentProvider;
    private currentWalkthrough?: WalkthroughData;
    private offscreenHtmlContent?: string;
    private placementMemory = new Map<string, PlacementState>(); // Unified placement memory
    private commentController?: vscode.CommentController;
    private webviewReady = false; // Track if webview has reported ready
    private commentThreads = new Map<string, vscode.CommentThread>(); // Track comment threads by comment ID

    constructor(
        private readonly _extensionUri: vscode.Uri,
        private readonly bus: Bus
    ) {
        this.md = this.setupMarkdownRenderer();
        this.diffContentProvider = new WalkthroughDiffContentProvider();

        // Register diff content provider
        this.bus.context.subscriptions.push(
            vscode.workspace.registerTextDocumentContentProvider('walkthrough-diff', this.diffContentProvider)
        );
    }

    private setupMarkdownRenderer(): MarkdownIt {
        const md = new MarkdownIt({
            html: true,
            linkify: true,
            typographer: true
        });

        // Custom renderer rule for file reference links
        const defaultRender = md.renderer.rules.link_open || function (tokens: any, idx: any, options: any, env: any, self: any) {
            return self.renderToken(tokens, idx, options);
        };

        md.renderer.rules.link_open = (tokens: any, idx: any, options: any, env: any, self: any) => {
            const token = tokens[idx];
            const href = token.attrGet('href');

            if (href && href.startsWith('symposium:')) {
                const linkKey = `link:${href}`;
                const placementState = this.placementMemory?.get(linkKey);

                token.attrSet('href', 'javascript:void(0)');
                token.attrSet('data-symposium-url', href);
                token.attrSet('class', 'file-ref');

                if (placementState?.isPlaced) {
                    token.attrSet('data-placement-state', 'placed');
                } else {
                    token.attrSet('data-placement-state', 'unplaced');
                }
            }

            return defaultRender(tokens, idx, options, env, self);
        };

        // Custom renderer for link close to add placement icons
        const defaultLinkClose = md.renderer.rules.link_close || function (tokens: any, idx: any, options: any, env: any, self: any) {
            return self.renderToken(tokens, idx, options);
        };

        md.renderer.rules.link_close = (tokens: any, idx: any, options: any, env: any, self: any) => {
            // Find the corresponding link_open token
            let openToken = null;
            for (let i = idx - 1; i >= 0; i--) {
                if (tokens[i].type === 'link_open') {
                    openToken = tokens[i];
                    break;
                }
            }

            if (openToken) {
                const href = openToken.attrGet('href');
                console.log('[RENDERER] Processing link_close for href:', href);
                if (href && href.startsWith('symposium:')) {
                    const linkKey = `link:${href}`;
                    const placementState = this.placementMemory?.get(linkKey);
                    const isPlaced = placementState?.isPlaced || false;

                    // Choose icon: 📍 for placed, 🔍 for unplaced
                    const icon = isPlaced ? '📍' : '🔍';
                    const action = isPlaced ? 'relocate' : 'place';
                    const title = isPlaced ? 'Relocate this link' : 'Place this link';

                    const result = `</a><button class="placement-icon" data-symposium-url="${href}" data-action="${action}" title="${title}">${icon}</button>`;
                    console.log('[RENDERER] Generated icon HTML:', result);
                    return result;
                }
            }

            return defaultLinkClose(tokens, idx, options, env, self);
        };

        return md;
    }

    private sanitizeHtml(html: string): string {
        // Basic HTML sanitization for VSCode webview context
        // Remove potentially dangerous content while preserving markdown-generated HTML
        return html.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
            .replace(/javascript:/gi, '')
            .replace(/on\w+="[^"]*"/gi, '');
    }

    private async handleWebviewMessage(message: any): Promise<void> {
        switch (message.command || message.type) {
            case 'clearWalkthrough':
                console.log('Walkthrough: clearWalkthrough command received');
                await this.clearWalkthrough();
                break;
            case 'openFile':
                console.log('Walkthrough: openFile command received:', message.symposiumUrl);
                await openSymposiumUrl(message.symposiumUrl, this.baseUri, this.placementMemory);
                // After placement, update the UI
                this.updateLinkPlacementUI(message.symposiumUrl);
                break;
            case 'relocateLink':
                console.log('Walkthrough: relocateLink command received:', message.symposiumUrl);
                await this.relocateLink(message.symposiumUrl);
                break;
            case 'action':
                console.log('Walkthrough: action received:', message.message);
                this.bus.log(`Action button clicked: ${message.message}`);

                // Send message to active AI terminal using Bus method
                await this.bus.sendTextToActiveTerminal(message.message);
                break;
            case 'showDiff':
                console.log('Walkthrough: showDiff command received:', message.filePath);
                await this.showFileDiff(message.filePath);
                break;
            case 'showComment':
                console.log('Walkthrough: showComment command received:', message.comment);
                await this.showComment(message.comment);
                break;
            case 'ready':
                console.log('Walkthrough webview ready');
                this.bus.log(`[WALKTHROUGH] Webview reported ready`);
                this.webviewReady = true;

                // Send any pending offscreen HTML content now that webview is ready
                if (this.offscreenHtmlContent && this._view) {
                    console.log('Webview ready - sending pending offscreen HTML content');
                    this.bus.log('[WALKTHROUGH] Webview ready - sending pending offscreen HTML content');
                    this._view.webview.postMessage({
                        type: 'showWalkthroughHtml',
                        content: this.offscreenHtmlContent
                    });
                }
                break;
        }
    }

    /**
     * Clear the walkthrough and dismiss any active comments
     */
    private async clearWalkthrough(): Promise<void> {
        console.log('[WALKTHROUGH] Clearing walkthrough and comments');

        // Clear any active comment threads
        if (this.commentController) {
            // Dispose all comment threads
            this.commentController.dispose();

            // Recreate the comment controller for future use
            this.commentController = vscode.comments.createCommentController(
                'symposium-walkthrough',
                'Dialectic Walkthrough'
            );

            // Set options to enable submit button
            this.commentController.options = {
                prompt: 'Discuss in Symposium...',
                placeHolder: 'Type your question or comment here...'
            };
        }

        // Clear placement memory
        this.placementMemory.clear();

        this.bus.log('Walkthrough cleared');
    }

    /**
     * Show comment using VSCode CommentController with context-aware file opening
     */
    private async showComment(comment: any): Promise<void> {
        console.log(`[WALKTHROUGH COMMENT] Starting showComment:`, comment);

        if (!comment.locations || comment.locations.length === 0) {
            vscode.window.showErrorMessage('Comment has no locations');
            return;
        }

        // Check if comment already exists (by ID)
        const existingThread = this.commentThreads.get(comment.id);
        if (existingThread) {
            await this.navigateToThread(existingThread);
            return;
        }

        // New comment - show location picker if ambiguous
        let selectedLocation;
        if (comment.locations.length === 1) {
            selectedLocation = comment.locations[0];
        } else {
            selectedLocation = await this.pickLocation(comment.locations, 'Choose the location for this comment');
            if (!selectedLocation) return; // User cancelled
        }

        // Create new comment
        const thread = await this.createCommentThread(selectedLocation.path, selectedLocation, comment);
        if (thread) {
            this.commentThreads.set(comment.id, thread);

            // Store placement state for ambiguous comments
            if (comment.locations.length > 1) {
                this.setPlacementState(comment.id, {
                    isPlaced: true,
                    chosenLocation: selectedLocation,
                    wasAmbiguous: true
                });

                // Update sidebar to show chosen location
                this.updateCommentDisplay(comment.id, selectedLocation);
            }

            await this.navigateToThread(thread);
        }
    }

    /**
     * Navigate to existing comment thread
     */
    private async navigateToThread(thread: vscode.CommentThread): Promise<void> {
        try {
            const document = await vscode.workspace.openTextDocument(thread.uri);
            const editor = await vscode.window.showTextDocument(document);

            const range = thread.range;
            if (range) {
                const position = range.start;
                editor.selection = new vscode.Selection(position, position);
                editor.revealRange(range);
            }

            // Ensure comment thread is visible
            thread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded;
        } catch (error) {
            console.error('[WALKTHROUGH] Failed to navigate to thread:', error);
        }
    }

    /**
     * Show relocation dialog for ambiguous comments
     */
    private async showRelocationDialog(comment: any, existingThread: vscode.CommentThread): Promise<void> {
        // Find current location
        const currentRange = existingThread.range;
        const currentPath = vscode.workspace.asRelativePath(existingThread.uri);
        const currentLine = currentRange ? currentRange.start.line + 1 : 0;

        // Build location options
        const locationItems = comment.locations.map((loc: any) => {
            const isCurrent = loc.path === currentPath && loc.start.line === currentLine;
            return {
                label: `${loc.path}:${loc.start.line}${isCurrent ? ' (current)' : ''}`,
                description: loc.content.substring(0, 80) + (loc.content.length > 80 ? '...' : ''),
                location: loc,
                isCurrent
            };
        });

        const selected = await vscode.window.showQuickPick(locationItems, {
            placeHolder: 'Choose location for this comment (current location marked)',
            matchOnDescription: true
        }) as { label: string; description: string; location: any; isCurrent: boolean } | undefined;

        if (!selected) return; // User cancelled

        if (selected.isCurrent) {
            // Navigate to existing
            await this.navigateToThread(existingThread);
        } else {
            // Relocate to new location
            existingThread.dispose();
            const newThread = await this.createCommentThread(selected.location.path, selected.location, comment);
            if (newThread) {
                this.commentThreads.set(comment.id, newThread);

                // Update placement state
                this.setPlacementState(comment.id, {
                    isPlaced: true,
                    chosenLocation: selected.location,
                    wasAmbiguous: true
                });

                // Update sidebar to show new chosen location
                this.updateCommentDisplay(comment.id, selected.location);
            }
        }
    }

    /**
     * Show location picker for ambiguous comments
     */
    private async pickLocation(locations: any[], placeholder: string): Promise<any> {
        const locationItems = locations.map((loc: any) => ({
            label: `${loc.path}:${loc.start.line}`,
            description: loc.content ?
                loc.content.substring(0, 80) + (loc.content.length > 80 ? '...' : '') :
                'No content available',
            location: loc
        }));

        const selected = await vscode.window.showQuickPick(locationItems, {
            placeHolder: placeholder,
            matchOnDescription: true
        }) as { label: string; description: string; location: any } | undefined;

        return selected?.location;
    }

    /**
     * Get set of files that appear in gitdiff sections of current walkthrough
     */
    private getFilesInCurrentGitDiff(): Set<string> {
        const filesInDiff = new Set<string>();

        if (!this.currentWalkthrough) return filesInDiff;

        const allSections = [
            ...(this.currentWalkthrough.introduction || []),
            ...(this.currentWalkthrough.highlights || []),
            ...(this.currentWalkthrough.changes || []),
            ...(this.currentWalkthrough.actions || [])
        ];

        for (const item of allSections) {
            if (typeof item === 'object' && 'files' in item) {
                // This is a GitDiffElement
                item.files.forEach((fileChange: FileChange) => {
                    filesInDiff.add(fileChange.path);
                });
            }
        }

        return filesInDiff;
    }

    /**
     * Create comment thread using VSCode CommentController
     */
    private async createCommentThread(filePath: string, location: any, comment: any): Promise<vscode.CommentThread | undefined> {
        console.log(`[WALKTHROUGH COMMENT] Creating comment thread for ${filePath}:${location.start.line}`);

        if (!this.baseUri) {
            console.error('[WALKTHROUGH COMMENT] No baseUri set');
            vscode.window.showErrorMessage('Cannot create comment: no base URI set');
            return undefined;
        }

        try {
            // Open the file first
            const uri = vscode.Uri.file(path.resolve(this.baseUri.fsPath, filePath));
            const document = await vscode.workspace.openTextDocument(uri);
            await vscode.window.showTextDocument(document);

            // Create comment controller if it doesn't exist
            if (!this.commentController) {
                this.commentController = vscode.comments.createCommentController(
                    'symposium-walkthrough',
                    'Dialectic Walkthrough Comments'
                );

                // Set options with custom reply command instead of text area
                this.commentController.options = {
                    // Remove prompt and placeHolder to eliminate embedded text area
                    // Add custom command for replies
                    // placeHolder: undefined, // Explicitly disable
                    // prompt: undefined
                };
            }

            // Create range for the comment (convert to 0-based)
            const startLine = Math.max(0, location.start.line - 1);
            const endLine = Math.max(0, (location.end?.line || location.start.line) - 1);
            const range = new vscode.Range(startLine, 0, endLine, Number.MAX_SAFE_INTEGER);

            // Create comment thread
            const thread = this.commentController.createCommentThread(uri, range, []);
            thread.label = 'Walkthrough Comment';
            thread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded; // Make visible immediately
            thread.canReply = false; // Disable default reply - we'll use custom commands

            // Add the comment content as the initial comment with reply button
            if (comment.comment && comment.comment.length > 0) {
                const commentText = comment.comment.join('\n\n');
                // Add reply button as a command link in the comment body
                const commentWithReply = `${commentText}\n\n---\n[Reply](command:symposium.replyToWalkthroughComment?${encodeURIComponent(JSON.stringify({
                    file: uri.fsPath,
                    range: { start: { line: startLine + 1 }, end: { line: endLine + 1 } },
                    comment: commentText
                }))})`;

                const commentBody = new vscode.MarkdownString(commentWithReply);
                commentBody.isTrusted = true; // Allow command execution
                commentBody.supportThemeIcons = true; // Enable theme icons if needed

                const vscodeComment: vscode.Comment = {
                    body: commentBody,
                    mode: vscode.CommentMode.Preview,
                    author: { name: 'Dialectic Walkthrough' },
                    timestamp: new Date()
                };
                thread.comments = [vscodeComment];
            }

            console.log(`[WALKTHROUGH COMMENT] Created comment thread at ${filePath}:${startLine + 1}`);
            return thread;

        } catch (error) {
            console.error(`[WALKTHROUGH COMMENT] Failed to create comment thread:`, error);
            vscode.window.showErrorMessage(`Failed to create comment: ${error}`);
            return undefined;
        }
    }

    /**
     * Update comment display in sidebar after location selection
     */
    private updateCommentDisplay(commentId: string, chosenLocation: any): void {
        if (!this._view) return;

        console.log(`[WALKTHROUGH] Updating comment display for ${commentId}:`, chosenLocation);

        // Send update to webview
        this._view.webview.postMessage({
            type: 'updateCommentDisplay',
            commentId: commentId,
            chosenLocation: chosenLocation
        });
    }

    // Placement state management methods

    /**
     * Get placement state for an item (link or comment)
     */
    private getPlacementState(key: string): PlacementState | undefined {
        return this.placementMemory.get(key);
    }

    /**
     * Set placement state for an item
     */
    private setPlacementState(key: string, state: PlacementState): void {
        this.placementMemory.set(key, state);
    }

    /**
     * Mark an item as placed with chosen location
     */
    private placeItem(key: string, location: any, wasAmbiguous: boolean): void {
        this.setPlacementState(key, {
            isPlaced: true,
            chosenLocation: location,
            wasAmbiguous
        });
    }

    /**
     * Mark an item as unplaced (for relocate functionality)
     */
    private unplaceItem(key: string): void {
        const currentState = this.getPlacementState(key);
        if (currentState) {
            this.setPlacementState(key, {
                ...currentState,
                isPlaced: false,
                chosenLocation: null
            });
        }
    }

    /**
     * Clear all placement memory (called when new walkthrough loads)
     */
    private clearPlacementMemory(): void {
        this.placementMemory.clear();
    }
    private async showFileDiff(filePath: string): Promise<void> {
        console.log(`[WALKTHROUGH DIFF] Starting showFileDiff for: ${filePath}`);

        if (!this.currentWalkthrough) {
            console.log('[WALKTHROUGH DIFF] ERROR: No current walkthrough data');
            vscode.window.showErrorMessage('No walkthrough data available');
            return;
        }

        // Find the file change in the walkthrough data
        let fileChange: FileChange | undefined;

        // Search through all sections for gitdiff elements
        const allSections = [
            ...(this.currentWalkthrough.introduction || []),
            ...(this.currentWalkthrough.highlights || []),
            ...(this.currentWalkthrough.changes || []),
            ...(this.currentWalkthrough.actions || [])
        ];

        for (const item of allSections) {
            if (typeof item === 'object' && 'files' in item) {
                // This is a GitDiffElement named field - {"files": FileChange[]}
                fileChange = item.files.find((fc: FileChange) => fc.path === filePath);
                if (fileChange) break;
            }
        }

        if (!fileChange) {
            console.log(`[WALKTHROUGH DIFF] ERROR: File not found in walkthrough: ${filePath}`);
            vscode.window.showErrorMessage(`File not found in walkthrough: ${filePath}`);
            return;
        }

        console.log(`[WALKTHROUGH DIFF] Found file change: ${fileChange.status}, ${fileChange.additions}+/${fileChange.deletions}-, ${fileChange.hunks.length} hunks`);

        try {
            // Get workspace folder
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const absolutePath = vscode.Uri.joinPath(workspaceFolder.uri, filePath);
            console.log(`[WALKTHROUGH DIFF] Resolved absolute path: ${absolutePath.toString()}`);

            // Get "after" content from current file
            const currentDocument = await vscode.workspace.openTextDocument(absolutePath);
            const modifiedContent = currentDocument.getText();
            console.log(`[WALKTHROUGH DIFF] Current file content length: ${modifiedContent.length} chars`);

            // Generate "before" content by reverse-applying hunks
            const originalContent = await this.generateOriginalContent(fileChange, modifiedContent);
            console.log(`[WALKTHROUGH DIFF] Generated original content length: ${originalContent.length} chars`);

            // Create URIs for diff content provider
            const originalUri = vscode.Uri.parse(`walkthrough-diff:${filePath}?original`);
            const modifiedUri = absolutePath; // Use actual file for "after" state
            console.log(`[WALKTHROUGH DIFF] Original URI: ${originalUri.toString()}`);
            console.log(`[WALKTHROUGH DIFF] Modified URI: ${modifiedUri.toString()}`);

            // Store original content in provider
            this.diffContentProvider.setContent(originalUri, originalContent);
            console.log('[WALKTHROUGH DIFF] Stored original content in provider');

            // Show diff using VSCode's native diff viewer with automatic highlighting
            console.log('[WALKTHROUGH DIFF] Calling vscode.diff command...');
            await vscode.commands.executeCommand('vscode.diff',
                originalUri,
                modifiedUri,
                `${filePath} (Walkthrough Diff)`
            );
            console.log('[WALKTHROUGH DIFF] vscode.diff command completed successfully');

        } catch (error) {
            console.error('[WALKTHROUGH DIFF] Failed to show file diff:', error);
            vscode.window.showErrorMessage(`Failed to show diff for ${filePath}`);
        }
    }

    /**
     * Generate original file content by reverse-applying diff hunks
     * Adapted from synthetic PR provider
     */
    private async generateOriginalContent(fileChange: FileChange, modifiedContent: string): Promise<string> {
        try {
            const modifiedLines = modifiedContent.split('\n');
            const originalLines: string[] = [];

            let modifiedIndex = 0;

            for (const hunk of fileChange.hunks) {
                // Add lines before this hunk (unchanged context)
                const contextStart = hunk.new_start - 1; // Convert to 0-based
                while (modifiedIndex < contextStart && modifiedIndex < modifiedLines.length) {
                    originalLines.push(modifiedLines[modifiedIndex]);
                    modifiedIndex++;
                }

                // Process hunk lines
                for (const line of hunk.lines) {
                    switch (line.line_type) {
                        case 'Context':
                            // Context lines appear in both versions
                            originalLines.push(line.content);
                            modifiedIndex++;
                            break;
                        case 'Removed':
                            // Removed lines were in original but not in modified
                            originalLines.push(line.content);
                            // Don't increment modifiedIndex
                            break;
                        case 'Added':
                            // Added lines are in modified but not in original
                            // Skip in original, but advance modified index
                            modifiedIndex++;
                            break;
                    }
                }
            }

            // Add any remaining lines after all hunks
            while (modifiedIndex < modifiedLines.length) {
                originalLines.push(modifiedLines[modifiedIndex]);
                modifiedIndex++;
            }

            return originalLines.join('\n');
        } catch (error) {
            console.error('[WALKTHROUGH DIFF] Failed to generate original content:', error);
            // Fallback to empty content for minimal diff display
            return '';
        }
    }

    public resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken,
    ) {
        console.log('WalkthroughWebviewProvider.resolveWebviewView called');
        this.bus.log('[WALKTHROUGH] resolveWebviewView called');
        console.log('Current offscreenHtmlContent length:', this.offscreenHtmlContent?.length || 0);
        this.bus.log(`[WALKTHROUGH] Current offscreenHtmlContent length: ${this.offscreenHtmlContent?.length || 0}`);

        this._view = webviewView;
        this.webviewReady = false; // Reset ready state for new webview

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this._extensionUri]
        };

        // Note: retainContextWhenHidden is not available on WebviewView
        // The webview will be recreated when hidden/shown, so we rely on
        // the offscreenHtmlContent mechanism to restore content

        // Handle messages from webview
        webviewView.webview.onDidReceiveMessage(
            message => this.handleWebviewMessage(message),
            undefined
        );

        console.log('Setting webview HTML');
        webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);
        console.log('Webview HTML set, waiting for ready message');

        // Note: We now wait for the 'ready' message from the webview before sending offscreen content
        // This ensures the webview is fully initialized and can properly handle the content
        if (!this.offscreenHtmlContent) {
            console.log('No offscreen HTML content to restore');
            this.bus.log('[WALKTHROUGH] No offscreen HTML content to restore');
        } else {
            console.log('Offscreen HTML content available, waiting for webview ready signal');
            this.bus.log('[WALKTHROUGH] Offscreen HTML content available, waiting for webview ready signal');
        }
    }

    public showWalkthroughHtml(htmlContent: string) {
        console.log('WalkthroughWebviewProvider.showWalkthroughHtml called with content length:', htmlContent.length);
        this.bus.log(`[WALKTHROUGH] showWalkthroughHtml called with ${htmlContent.length} chars`);
        this.bus.log(`[WALKTHROUGH] HTML content received from MCP server:`);
        this.bus.log(htmlContent);

        // Always store the content so it persists across webview dispose/recreate cycles
        this.offscreenHtmlContent = htmlContent;

        if (this._view) {
            console.log('Webview exists, showing and posting HTML content');
            this.bus.log(`[WALKTHROUGH] Webview exists, posting message to webview`);
            this._view.show?.(true);

            // Only send immediately if webview is ready, otherwise wait for ready message
            if (this.webviewReady) {
                console.log('Webview is ready, sending HTML content immediately');
                this.bus.log(`[WALKTHROUGH] Webview is ready, sending HTML content immediately`);
                this._view.webview.postMessage({
                    type: 'showWalkthroughHtml',
                    content: htmlContent
                });
            } else {
                console.log('Webview not ready yet, content will be sent when ready message is received');
                this.bus.log(`[WALKTHROUGH] Webview not ready yet, content will be sent when ready message is received`);
            }
        } else {
            console.log('No webview available, content stored for when webview becomes available');
            this.bus.log(`[WALKTHROUGH] No webview available, content stored as pending`);
        }
    }

    public showWalkthrough(walkthrough: WalkthroughData) {
        console.log('WalkthroughWebviewProvider.showWalkthrough called with:', walkthrough);

        // Store walkthrough data for diff functionality
        this.currentWalkthrough = walkthrough;

        // Clear placement memory for new walkthrough
        this.clearPlacementMemory();

        // Clear all existing comments
        this.clearAllComments();

        if (this._view) {
            console.log('Webview exists, showing and posting message');
            this._view.show?.(true);

            // Pre-render markdown content
            const processedWalkthrough = this.processWalkthroughMarkdown(walkthrough);

            this._view.webview.postMessage({
                type: 'walkthrough',
                data: processedWalkthrough
            });
            console.log('Message posted to webview');

            // Auto-place unambiguous comments using original walkthrough data
            this.autoPlaceUnambiguousComments(walkthrough);
        } else {
            console.log('ERROR: No webview available');
        }
    }

    /**
     * Handle comment submission from VSCode (replies to walkthrough comments)
     */
    public async handleCommentSubmission(reply: vscode.CommentReply): Promise<void> {
        const newComment: vscode.Comment = {
            body: new vscode.MarkdownString(reply.text),
            mode: vscode.CommentMode.Preview,
            author: {
                name: 'User'
            },
            timestamp: new Date()
        };

        reply.thread.comments = [...reply.thread.comments, newComment];

        // Ensure the thread can accept more replies
        reply.thread.canReply = true;

        // Send to active AI shell with context
        await this.sendCommentToShell(reply.text, reply.thread);
    }

    /**
     * Send comment reply to active AI shell with context
     */
    private async sendCommentToShell(text: string, thread: vscode.CommentThread): Promise<void> {
        try {
            if (!thread.range) {
                console.error('[WALKTHROUGH] Comment thread has no range');
                return;
            }

            const uri = thread.uri;
            const lineNumber = thread.range.start.line + 1; // Convert to 1-based
            const filePath = vscode.workspace.asRelativePath(uri);

            // Use new consolidated sendToActiveTerminal method
            const referenceData = {
                file: filePath,
                line: lineNumber,
                selection: undefined,
                user_comment: text
            };

            await this.bus.sendToActiveTerminal(referenceData, { includeNewline: true });
            this.bus.log(`Comment reply sent as compact reference for ${filePath}:${lineNumber}`);
        } catch (error) {
            console.error('[WALKTHROUGH] Error sending comment to shell:', error);
            this.bus.log(`Error sending comment: ${error}`);
        }
    }

    /**
     * Clear all existing comment threads
     */
    private clearAllComments(): void {
        if (this.commentController) {
            console.log('[WALKTHROUGH] Clearing all existing comments');
            this.commentController.dispose();
            this.commentController = undefined;
        }
        this.commentThreads.clear();
    }

    /**
     * Auto-place comments that have unambiguous locations (exactly one location)
     */
    private async autoPlaceUnambiguousComments(walkthrough: WalkthroughData): Promise<void> {
        const allSections = [
            ...(walkthrough.introduction || []),
            ...(walkthrough.highlights || []),
            ...(walkthrough.changes || []),
            ...(walkthrough.actions || [])
        ];

        for (const item of allSections) {
            if (typeof item === 'object' && 'comment' in item) {
                const commentItem = item as any;
                if (commentItem.locations && commentItem.locations.length === 1) {
                    // Auto-place unambiguous comments
                    const thread = await this.createCommentThread(commentItem.locations[0].path, commentItem.locations[0], commentItem);
                    if (thread) {
                        this.commentThreads.set(commentItem.id, thread);
                    }
                }
            }
        }
    }

    public setBaseUri(baseUri: string) {
        this.baseUri = vscode.Uri.file(baseUri);
    }

    private processWalkthroughMarkdown(walkthrough: WalkthroughData): WalkthroughData {
        const processSection = (items?: WalkthroughElement[]) => {
            if (!items) return items;
            return items.map(item => {
                if (typeof item === 'string') {
                    // Process plain markdown strings
                    return this.sanitizeHtml(this.md.render(item));
                } else if (typeof item === 'object' && 'files' in item) {
                    // Handle GitDiffElement named field - {"files": FileChange[]}
                    return item; // Keep as-is, will be handled in rendering
                }
                return item;
            });
        };

        return {
            introduction: processSection(walkthrough.introduction),
            highlights: processSection(walkthrough.highlights),
            changes: processSection(walkthrough.changes),
            actions: processSection(walkthrough.actions)
        };
    }

    private async relocateLink(symposiumUrl: string): Promise<void> {
        // Remove the current placement to force re-disambiguation
        const linkKey = `link:${symposiumUrl}`;
        this.placementMemory?.delete(linkKey);

        // Open the link again - this will show disambiguation
        await openSymposiumUrl(symposiumUrl, this.baseUri, this.placementMemory);

        // Update UI after relocation
        this.updateLinkPlacementUI(symposiumUrl);
    }

    private updateLinkPlacementUI(symposiumUrl: string): void {
        if (!this._view) return;

        const linkKey = `link:${symposiumUrl}`;
        const placementState = this.placementMemory?.get(linkKey);
        const isPlaced = placementState?.isPlaced || false;

        console.log(`[Walkthrough] Updating UI for ${symposiumUrl}: isPlaced=${isPlaced}, placementState=`, placementState);

        // Send update to webview
        this._view.webview.postMessage({
            type: 'updateLinkPlacement',
            symposiumUrl: symposiumUrl,
            isPlaced: isPlaced
        });
    }

    private _getHtmlForWebview(_webview: vscode.Webview) {
        const nonce = crypto.randomBytes(16).toString('base64');

        let html = `<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${nonce}' https://cdn.jsdelivr.net;">
                <title>Walkthrough</title>
                <style>
                    body {
                        font-family: var(--vscode-font-family);
                        font-size: var(--vscode-font-size);
                        color: var(--vscode-foreground);
                        background-color: var(--vscode-editor-background);
                        margin: 0;
                        padding: 16px;
                        line-height: 1.5;
                    }
                    .walkthrough-header {
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        margin-bottom: 20px;
                        padding-bottom: 12px;
                        border-bottom: 1px solid var(--vscode-panel-border);
                    }
                    .walkthrough-title {
                        font-size: 1.2em;
                        font-weight: 600;
                        color: var(--vscode-textLink-foreground);
                    }
                    .clear-button {
                        background-color: var(--vscode-button-secondaryBackground);
                        color: var(--vscode-button-secondaryForeground);
                        border: none;
                        padding: 6px 12px;
                        border-radius: 3px;
                        cursor: pointer;
                        font-size: 0.9em;
                    }
                    .clear-button:hover {
                        background-color: var(--vscode-button-secondaryHoverBackground);
                    }
                    .section {
                        margin-bottom: 24px;
                    }
                    .section-title {
                        font-size: 1.1em;
                        font-weight: 600;
                        color: var(--vscode-textLink-foreground);
                        margin-bottom: 12px;
                        border-bottom: 1px solid var(--vscode-panel-border);
                        padding-bottom: 4px;
                    }
                    .content-item {
                        margin-bottom: 8px;
                        padding: 4px 0;
                    }
                    .action-button {
                        background-color: var(--vscode-button-background);
                        color: var(--vscode-button-foreground);
                        border: none;
                        padding: 8px 16px;
                        border-radius: 4px;
                        cursor: pointer;
                        margin: 4px 0;
                        font-size: 0.9em;
                    }
                    .action-button:hover {
                        background-color: var(--vscode-button-hoverBackground);
                    }
                    .action-description {
                        font-size: 0.85em;
                        color: var(--vscode-descriptionForeground);
                        margin-top: 4px;
                    }
                    pre {
                        background-color: var(--vscode-textCodeBlock-background);
                        padding: 12px;
                        border-radius: 4px;
                        overflow-x: auto;
                        font-family: var(--vscode-editor-font-family);
                    }
                    code {
                        background-color: var(--vscode-textCodeBlock-background);
                        padding: 2px 4px;
                        border-radius: 2px;
                        font-family: var(--vscode-editor-font-family);
                    }
                    .empty-state {
                        text-align: center;
                        color: var(--vscode-descriptionForeground);
                        font-style: italic;
                        padding: 32px 16px;
                    }
                    .gitdiff-container {
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        margin: 8px 0;
                    }
                    .file-diff {
                        border-bottom: 1px solid var(--vscode-panel-border);
                    }
                    .file-diff:last-child {
                        border-bottom: none;
                    }
                    .file-header {
                        display: flex;
                        align-items: center;
                        padding: 8px 12px;
                        background-color: var(--vscode-editor-background);
                        font-family: var(--vscode-editor-font-family);
                        font-size: 0.9em;
                    }
                    .file-path {
                        flex: 1;
                        font-weight: 500;
                    }
                    .clickable-file {
                        cursor: pointer;
                        color: var(--vscode-textLink-foreground);
                        text-decoration: underline;
                    }
                    .clickable-file:hover {
                        color: var(--vscode-textLink-activeForeground);
                    }
                    .file-stats {
                        margin: 0 12px;
                        color: var(--vscode-descriptionForeground);
                        font-size: 0.85em;
                    }
                    .comment-item {
                        display: flex;
                        align-items: flex-start;
                        padding: 8px;
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        cursor: pointer;
                        background-color: var(--vscode-editor-background);
                    }
                    .comment-item:hover {
                        background-color: var(--vscode-list-hoverBackground);
                    }
                    .comment-icon {
                        margin-right: 8px;
                        font-size: 16px;
                    }
                    .comment-content {
                        flex: 1;
                    }
                    .comment-locations {
                        font-weight: 500;
                        color: var(--vscode-textLink-foreground);
                        margin-bottom: 4px;
                    }
                    .comment-location {
                        font-family: var(--vscode-editor-font-family);
                        font-size: 0.9em;
                    }
                    .comment-text {
                        color: var(--vscode-foreground);
                        font-size: 0.9em;
                    }
                    
                    /* Placement UI styles */
                    .file-ref {
                        cursor: pointer;
                        text-decoration: none;
                        display: inline-flex;
                        align-items: center;
                        gap: 4px;
                        color: var(--vscode-textLink-foreground);
                        border-bottom: 1px solid var(--vscode-textLink-foreground);
                    }
                    
                    .file-ref:hover {
                        color: var(--vscode-textLink-activeForeground);
                        border-bottom-color: var(--vscode-textLink-activeForeground);
                    }
                    
                    .placement-icon {
                        background: none;
                        border: none;
                        cursor: pointer;
                        padding: 0;
                        font-size: 0.9em;
                        opacity: 0.8;
                        margin-left: 2px;
                    }
                    
                    .placement-icon:hover {
                        opacity: 1;
                    }
                    
                    /* Mermaid diagram styles */
                    .mermaid-container {
                        margin: 16px 0;
                        text-align: center;
                        background-color: var(--vscode-editor-background);
                        border: 1px solid var(--vscode-panel-border);
                        border-radius: 4px;
                        padding: 16px;
                    }
                    
                    .mermaid {
                        background-color: transparent !important;
                    }
                </style>
            </head>
            <body>
                <div class="walkthrough-header">
                    <div class="walkthrough-title">Code Walkthrough</div>
                    <button class="clear-button" id="clear-walkthrough">Clear</button>
                </div>
                <div id="content">
                    <div class="empty-state">No walkthrough loaded</div>
                </div>
                <script src="https://cdn.jsdelivr.net/npm/mermaid@10.9.1/dist/mermaid.min.js"></script>
                <script nonce="${nonce}">
                    console.log('Walkthrough webview JavaScript loaded');
                    const vscode = acquireVsCodeApi();
                    console.log('VSCode API acquired');
                    
                    // State management for webview interaction persistence
                    function saveOffscreenState() {
                        const state = {
                            scrollPosition: window.scrollY,
                            timestamp: Date.now()
                        };
                        vscode.setState(state);
                        console.log('[STATE] Saved offscreen state:', state);
                    }
                    
                    function restoreOffscreenState() {
                        const state = vscode.getState();
                        if (state && state.scrollPosition !== undefined) {
                            console.log('[STATE] Restoring offscreen state:', state);
                            // Use requestAnimationFrame to ensure content is rendered first
                            requestAnimationFrame(() => {
                                window.scrollTo(0, state.scrollPosition);
                                console.log('[STATE] Scroll position restored to:', state.scrollPosition);
                            });
                        } else {
                            console.log('[STATE] No offscreen state to restore');
                        }
                    }
                    
                    // Save state on scroll (throttled to avoid excessive saving)
                    let scrollTimeout;
                    window.addEventListener('scroll', () => {
                        if (scrollTimeout) clearTimeout(scrollTimeout);
                        scrollTimeout = setTimeout(saveOffscreenState, 100);
                    });
                    
                    // Initialize mermaid with VSCode theme colors
                    function initializeMermaidWithTheme() {
                        if (typeof mermaid === 'undefined') {
                            console.warn('Mermaid library not loaded');
                            return;
                        }
                        
                        // Get current VSCode theme colors
                        const computedStyles = getComputedStyle(document.body);
                        const primaryColor = computedStyles.getPropertyValue('--vscode-textLink-foreground').trim() || '#0078d4';
                        const primaryTextColor = computedStyles.getPropertyValue('--vscode-foreground').trim() || '#1f1f1f';
                        const primaryBorderColor = computedStyles.getPropertyValue('--vscode-panel-border').trim() || '#e1e1e1';
                        const lineColor = computedStyles.getPropertyValue('--vscode-panel-border').trim() || '#666666';
                        const secondaryColor = computedStyles.getPropertyValue('--vscode-editor-background').trim() || '#f3f3f3';
                        const backgroundColor = computedStyles.getPropertyValue('--vscode-editor-background').trim() || '#ffffff';
                        
                        console.log('VSCode theme colors:', {
                            primaryColor,
                            primaryTextColor,
                            primaryBorderColor,
                            lineColor,
                            secondaryColor,
                            backgroundColor
                        });
                        
                        mermaid.initialize({
                            startOnLoad: false,
                            theme: 'base',
                            themeVariables: {
                                primaryColor: primaryColor,
                                primaryTextColor: primaryTextColor,
                                primaryBorderColor: primaryBorderColor,
                                lineColor: lineColor,
                                secondaryColor: secondaryColor,
                                tertiaryColor: backgroundColor,
                                background: backgroundColor,
                                mainBkg: backgroundColor,
                                secondBkg: secondaryColor
                            }
                        });
                        console.log('Mermaid initialized with VSCode theme colors');
                    }
                    
                    initializeMermaidWithTheme();
                    
                    // Function to process mermaid diagrams in content
                    async function processMermaidDiagrams() {
                        console.log('[MERMAID] Processing mermaid diagrams');
                        console.log('[MERMAID] Mermaid library available:', typeof mermaid !== 'undefined');
                        
                        const mermaidElements = document.querySelectorAll('mermaid');
                        console.log('[MERMAID] Found', mermaidElements.length, 'mermaid elements');
                        
                        if (mermaidElements.length === 0) {
                            console.log('[MERMAID] No mermaid elements found, skipping');
                            return;
                        }
                        
                        if (typeof mermaid === 'undefined') {
                            console.error('[MERMAID] Mermaid library not loaded!');
                            return;
                        }
                        
                        // Process each mermaid element
                        mermaidElements.forEach((element, index) => {
                            try {
                                const diagramContent = element.textContent || element.innerHTML;
                                console.log('[MERMAID] Processing diagram', index);
                                console.log('[MERMAID] Raw content:', diagramContent);
                                
                                // Clean and validate the diagram content
                                const cleanContent = diagramContent.trim();
                                if (!cleanContent) {
                                    console.warn('[MERMAID] Empty diagram content, skipping element', index);
                                    return;
                                }
                                
                                // Basic validation - check if it looks like a mermaid diagram
                                const validPrefixes = ['flowchart', 'graph', 'sequenceDiagram', 'classDiagram', 'stateDiagram', 'journey', 'gitgraph', 'pie'];
                                const hasValidPrefix = validPrefixes.some(prefix => cleanContent.toLowerCase().startsWith(prefix.toLowerCase()));
                                
                                if (!hasValidPrefix) {
                                    console.warn('[MERMAID] Content does not appear to be a valid mermaid diagram, skipping:', cleanContent.substring(0, 50) + '...');
                                    return;
                                }
                                
                                console.log('[MERMAID] Valid diagram content detected:', cleanContent.substring(0, 50) + '...');
                                
                                // Create container div
                                const container = document.createElement('div');
                                container.className = 'mermaid-container';
                                
                                // Create mermaid div
                                const mermaidDiv = document.createElement('div');
                                mermaidDiv.className = 'mermaid';
                                mermaidDiv.textContent = cleanContent;
                                
                                container.appendChild(mermaidDiv);
                                
                                // Replace original mermaid element
                                element.parentNode.replaceChild(container, element);
                                console.log('[MERMAID] Replaced element', index, 'with container');
                            } catch (error) {
                                console.error('[MERMAID] Error processing element', index, ':', error);
                            }
                        });
                        
                        try {
                            console.log('[MERMAID] About to call mermaid.run()');
                            // Render all mermaid diagrams
                            await mermaid.run();
                            console.log('[MERMAID] All diagrams rendered successfully');
                        } catch (error) {
                            console.error('[MERMAID] Error rendering diagrams:', error);
                            console.error('[MERMAID] Error stack:', error.stack);
                        }
                    }
                    
                    // Handle clicks on dialectic URLs and placement icons
                    document.addEventListener('click', function(event) {
                        const target = event.target;
                        if (!target) return;
                        
                        // Handle placement icon clicks
                        if (target.classList.contains('placement-icon')) {
                            event.preventDefault();
                            const symposiumUrl = target.getAttribute('data-symposium-url');
                            const action = target.getAttribute('data-action');
                            
                            console.log('[Walkthrough] Placement icon clicked:', symposiumUrl, 'action:', action);
                            
                            if (action === 'relocate') {
                                vscode.postMessage({
                                    command: 'relocateLink',
                                    symposiumUrl: symposiumUrl
                                });
                            } else {
                                vscode.postMessage({
                                    command: 'openFile',
                                    symposiumUrl: symposiumUrl
                                });
                            }
                            return;
                        }
                        
                        // Check if clicked element or parent has dialectic URL (link text clicked)
                        let element = target;
                        while (element && element !== document) {
                            const symposiumUrl = element.getAttribute('data-symposium-url');
                            if (symposiumUrl && element.classList.contains('file-ref')) {
                                event.preventDefault();
                                console.log('[Walkthrough] Link text clicked - navigating:', symposiumUrl);
                                
                                vscode.postMessage({
                                    command: 'openFile',
                                    symposiumUrl: symposiumUrl
                                });
                                return;
                            }
                            element = element.parentElement;
                        }
                    });
                    
                    // Function to add placement icons to all dialectic links
                    function addPlacementIcons() {
                        console.log('[ICONS] Adding placement icons to dialectic links');
                        const dialecticLinks = document.querySelectorAll('a[data-symposium-url]');
                        console.log('[ICONS] Found', dialecticLinks.length, 'dialectic links');
                        
                        dialecticLinks.forEach((link, index) => {
                            const symposiumUrl = link.getAttribute('data-symposium-url');
                            console.log('[ICONS] Processing link', index, 'URL:', symposiumUrl);
                            
                            // Check if ANY placement icon already exists for this URL
                            const existingIcons = document.querySelectorAll('.placement-icon[data-symposium-url="' + symposiumUrl + '"]');
                            if (existingIcons.length > 0) {
                                console.log('[ICONS] Icon already exists for URL:', symposiumUrl, 'count:', existingIcons.length);
                                return;
                            }
                            
                            // Create placement icon
                            const icon = document.createElement('button');
                            icon.className = 'placement-icon';
                            icon.setAttribute('data-symposium-url', symposiumUrl);
                            icon.setAttribute('data-action', 'place');
                            icon.setAttribute('title', 'Place this link');
                            icon.textContent = '🔍'; // Default to search icon
                            
                            // Insert icon after the link
                            link.parentNode.insertBefore(icon, link.nextSibling);
                            console.log('[ICONS] Added icon for link', index);
                        });
                    }

                    // Function to update link rendering after placement changes
                    function updateLinkPlacement(symposiumUrl, isPlaced) {
                        console.log('[PLACEMENT] updateLinkPlacement called with:', symposiumUrl, 'isPlaced:', isPlaced);
                        
                        // Debug: show all placement icons in the DOM
                        const allIcons = document.querySelectorAll('.placement-icon');
                        console.log('[PLACEMENT] All placement icons in DOM:', allIcons.length);
                        allIcons.forEach((icon, i) => {
                            console.log('[PLACEMENT] Icon ' + i + ': data-symposium-url="' + icon.getAttribute('data-symposium-url') + '" text="' + icon.textContent + '"');
                        });
                        
                        // Update placement icons
                        const icons = document.querySelectorAll('.placement-icon[data-symposium-url="' + symposiumUrl + '"]');
                        console.log('[PLACEMENT] Found', icons.length, 'icons to update for URL:', symposiumUrl);
                        
                        icons.forEach((icon, index) => {
                            console.log('[PLACEMENT] Updating icon', index, 'current text:', icon.textContent);
                            if (isPlaced) {
                                icon.textContent = '📍';
                                icon.setAttribute('data-action', 'relocate');
                                icon.setAttribute('title', 'Relocate this link');
                                console.log('[PLACEMENT] Set icon to 📍 (relocate)');
                            } else {
                                icon.textContent = '🔍';
                                icon.setAttribute('data-action', 'place');
                                icon.setAttribute('title', 'Place this link');
                                console.log('[PLACEMENT] Set icon to 🔍 (place)');
                            }
                        });
                        
                        // Update link data attributes
                        const links = document.querySelectorAll('.file-ref[data-symposium-url="' + symposiumUrl + '"]');
                        console.log('[PLACEMENT] Found', links.length, 'links to update');
                        links.forEach(link => {
                            link.setAttribute('data-placement-state', isPlaced ? 'placed' : 'unplaced');
                        });
                    }
                    
                    // Function to update comment display after location selection
                    function updateCommentDisplay(commentId, chosenLocation) {
                        console.log('[COMMENT] updateCommentDisplay called with:', commentId, chosenLocation);
                        
                        // Find the comment item by ID
                        const commentItems = document.querySelectorAll('.comment-item');
                        commentItems.forEach(item => {
                            try {
                                const commentData = JSON.parse(decodeURIComponent(item.dataset.comment));
                                if (commentData.id === commentId) {
                                    // Update the location display
                                    const locationElement = item.querySelector('.comment-locations .comment-location');
                                    if (locationElement) {
                                        locationElement.textContent = chosenLocation.path + ':' + chosenLocation.start.line;
                                        console.log('[COMMENT] Updated location display for', commentId);
                                    }
                                }
                            } catch (e) {
                                console.error('[COMMENT] Error parsing comment data:', e);
                            }
                        });
                    }
                    
                    // HTML parsing is now done server-side - no client-side parsing needed
                    
                    function renderMarkdown(text) {
                        return text; // Content is already rendered HTML
                    }
                    
                    function renderSection(title, items) {
                        if (!items || items.length === 0) return '';
                        
                        let html = '<div class="section">';
                        html += '<div class="section-title">' + title + '</div>';
                        
                        items.forEach(item => {
                            if (typeof item === 'string') {
                                // ResolvedMarkdownElement now serialized as plain string
                                html += '<div class="content-item">' + renderMarkdown(item) + '</div>';
                            } else if (typeof item === 'object' && 'locations' in item && 'comment' in item) {
                                // ResolvedComment object sent directly (not wrapped)
                                html += '<div class="content-item">';
                                html += '<div class="comment-item" data-comment="' + encodeURIComponent(JSON.stringify(item)) + '">';
                                html += '<div class="comment-icon">💬</div>';
                                html += '<div class="comment-content">';
                                
                                // Smart location display for ambiguous comments
                                html += '<div class="comment-locations">';
                                if (item.locations.length === 1) {
                                    // Unambiguous - show exact location
                                    const loc = item.locations[0];
                                    html += '<span class="comment-location">' + loc.path + ':' + loc.start.line + '</span>';
                                } else {
                                    // Ambiguous - check if all same file
                                    const firstFile = item.locations[0].path;
                                    const allSameFile = item.locations.every(loc => loc.path === firstFile);
                                    
                                    if (allSameFile) {
                                        html += '<span class="comment-location">' + firstFile + ' 🔍</span>';
                                    } else {
                                        html += '<span class="comment-location">(' + item.locations.length + ' possible locations) 🔍</span>';
                                    }
                                }
                                html += '</div>';
                                
                                if (item.comment && item.comment.length > 0) {
                                    html += '<div class="comment-text">' + item.comment.join(' ') + '</div>';
                                }
                                html += '</div>';
                                html += '</div>';
                                html += '</div>';
                            } else if (typeof item === 'object' && 'files' in item) {
                                // GitDiffElement named field - {"files": FileChange[]}
                                html += '<div class="content-item">';
                                html += '<div class="gitdiff-container">';
                                item.files.forEach(fileChange => {
                                    html += '<div class="file-diff">';
                                    html += '<div class="file-header">';
                                    html += '<span class="file-path clickable-file" data-file-path="' + fileChange.path + '">' + fileChange.path + '</span>';
                                    html += '<span class="file-stats">+' + fileChange.additions + ' -' + fileChange.deletions + '</span>';
                                    html += '</div>';
                                    html += '</div>';
                                });
                                html += '</div>';
                                html += '</div>';
                            } else if (item.Action && item.Action.button) {
                                // Action wrapper object
                                html += '<div class="content-item">';
                                html += '<button class="action-button" data-tell-agent="' + 
                                       (item.Action.tell_agent || '').replace(/"/g, '&quot;') + '">' + 
                                       item.Action.button + '</button>';
                                if (item.Action.description) {
                                    html += '<div class="action-description">' + item.Action.description + '</div>';
                                }
                                html += '</div>';
                            } else if (item.button) {
                                // Direct action object with button property
                                html += '<div class="content-item">';
                                html += '<button class="action-button" data-tell-agent="' + 
                                       (item.tell_agent || '').replace(/"/g, '&quot;') + '">' + 
                                       item.button + '</button>';
                                if (item.description) {
                                    html += '<div class="action-description">' + item.description + '</div>';
                                }
                                html += '</div>';
                            }
                        });
                        
                        html += '</div>';
                        return html;
                    }
                    
                    function handleAction(message) {
                        if (message) {
                            vscode.postMessage({
                                type: 'action',
                                message: message
                            });
                        }
                    }

                    // Add event listener for action button clicks (CSP-compliant)
                    document.addEventListener('click', (event) => {
                        if (event.target.id === 'clear-walkthrough') {
                            // Clear walkthrough and dismiss comments
                            document.getElementById('content').innerHTML = '<div class="empty-state">No walkthrough loaded</div>';
                            vscode.postMessage({
                                type: 'clearWalkthrough'
                            });
                        } else if (event.target.tagName === 'BUTTON' && 
                            event.target.classList.contains('action-button') && 
                            event.target.dataset.tellAgent) {
                            handleAction(event.target.dataset.tellAgent);
                        } else if (event.target.classList.contains('clickable-file') && 
                                   event.target.dataset.filePath) {
                            vscode.postMessage({
                                type: 'showDiff',
                                filePath: event.target.dataset.filePath
                            });
                        } else if (event.target.closest('.comment-item')) {
                            const commentItem = event.target.closest('.comment-item');
                            const commentData = JSON.parse(decodeURIComponent(commentItem.dataset.comment));
                            vscode.postMessage({
                                type: 'showComment',
                                comment: commentData
                            });
                        }
                    });
                    
                    window.addEventListener('message', event => {
                        console.log('[WEBVIEW] Received message:', event.data);
                        const message = event.data;
                        if (message.type === 'walkthrough') {
                            console.log('[WALKTHROUGH] Processing message with data:', message.data);
                            const data = message.data;
                            
                            console.log('[SECTIONS] Walkthrough sections:', {
                                introduction: data.introduction?.length || 0,
                                highlights: data.highlights?.length || 0, 
                                changes: data.changes?.length || 0,
                                actions: data.actions?.length || 0
                            });
                            
                            let html = '';
                            
                            html += renderSection('Introduction', data.introduction);
                            html += renderSection('Highlights', data.highlights);
                            html += renderSection('Changes', data.changes);
                            html += renderSection('Actions', data.actions);
                            
                            console.log('[HTML] Generated HTML length:', html.length);
                            const finalHtml = html || '<div class="empty-state">Empty walkthrough</div>';
                            console.log('[UPDATE] Setting innerHTML to content element');
                            
                            const contentElement = document.getElementById('content');
                            if (contentElement) {
                                contentElement.innerHTML = finalHtml;
                                console.log('[SUCCESS] Content updated successfully');
                                
                                // Add placement icons to all dialectic links
                                addPlacementIcons();
                                
                                // Process mermaid diagrams
                                processMermaidDiagrams();
                                
                                // Restore user interaction state (scroll position, etc.)
                                restoreOffscreenState();
                            } else {
                                console.error('[ERROR] Content element not found!');
                            }
                        } else if (message.type === 'updateLinkPlacement') {
                            console.log('[PLACEMENT] Updating link placement:', message.symposiumUrl, 'isPlaced:', message.isPlaced);
                            updateLinkPlacement(message.symposiumUrl, message.isPlaced);
                        } else if (message.type === 'updateCommentDisplay') {
                            console.log('[COMMENT] Updating comment display:', message.commentId, message.chosenLocation);
                            updateCommentDisplay(message.commentId, message.chosenLocation);
                        } else if (message.type === 'showWalkthroughHtml') {
                            console.log('[HTML] Showing walkthrough HTML content, length:', message.content.length);
                            console.log('[HTML] Content preview:', message.content.substring(0, 200) + '...');
                            
                            const contentElement = document.getElementById('content');
                            if (contentElement) {
                                console.log('[HTML] Setting HTML content directly (server-rendered)');
                                contentElement.innerHTML = message.content;
                                console.log('[SUCCESS] HTML content set');
                                
                                // Add placement icons to all dialectic links
                                addPlacementIcons();
                                
                                // Process mermaid diagrams in the HTML content
                                processMermaidDiagrams();
                                
                                // Restore user interaction state (scroll position, etc.)
                                restoreOffscreenState();
                            } else {
                                console.error('[ERROR] Content element not found!');
                            }
                        } else {
                            console.log('[IGNORE] Ignoring message type:', message.type);
                        }
                    });
                    
                    // Notify extension that webview is ready
                    console.log('[WEBVIEW] Sending ready message to extension');
                    vscode.postMessage({
                        command: 'ready'
                    });
                    console.log('[WEBVIEW] Ready message sent');
                </script>
            </body>
            </html>`;

        this.bus.log(`-----------------------------------------`);
        this.bus.log(`WEBVIEW HTML FOLLOWS:`);
        this.bus.log(html);
        this.bus.log(`-----------------------------------------`);

        return html;
    }

    dispose() {
        if (this.commentController) {
            this.commentController.dispose();
            this.commentController = undefined;
        }
    }
}
