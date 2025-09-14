import * as vscode from 'vscode';
import * as path from 'path';
import * as crypto from 'crypto';
import { SyntheticPRProvider } from './syntheticPRProvider';
import { WalkthroughWebviewProvider } from './walkthroughWebview';
import { StructuredLogger } from './structuredLogger';

interface IPCMessage {
    shellPid: number;
    type: string;
    payload: any;
    id: string;
}

interface LogPayload {
    level: 'info' | 'error' | 'debug';
    message: string;
}

interface ResolveSymbolPayload {
    name: string;
}

interface FindReferencesPayload {
    symbol: SymbolDef;
}

interface ResponsePayload {
    success: boolean;
    data?: any;
    error?: string;
}

interface SyntheticPRPayload {
    review_id: string;
    title: string;
    description: any;
    commit_range: string;
    files_changed: any[];
    comment_threads: any[];
    status: string;
}

interface StoreReferencePayload {
    key: string;
    value: any;
}

interface PresentWalkthroughPayload {
    content: string;
    base_uri: string;
}

interface TaskspaceRollCallPayload {
    taskspace_uuid: string;
}

interface RegisterTaskspaceWindowPayload {
    window_title: string;
    taskspace_uuid: string;
}

interface UserFeedback {
    feedback_type: 'comment' | 'complete_review';
    review_id: string;
    comment_text?: string;
    file_path?: string;
    line_number?: number;
    completion_action?: 'request_changes' | 'checkpoint' | 'return';
    additional_notes?: string;
}

interface SymbolDef {
    name: string;
    kind?: string;
    definedAt: FileRange;
}

interface FileRange {
    path: string;
    start: Position;
    end: Position;
}

interface Position {
    line: number;
    column: number;
}

function getCurrentTaskspaceUuid(): string | null {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath;
    const taskUuidPattern = /^task-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i;

    let currentDir = workspaceRoot;
    while (currentDir !== path.dirname(currentDir)) {
        const dirName = path.basename(currentDir);
        const match = dirName.match(taskUuidPattern);

        if (match) {
            const taskspaceJsonPath = path.join(currentDir, 'taskspace.json');
            const fs = require('fs');
            if (fs.existsSync(taskspaceJsonPath)) {
                return match[1];
            }
        }

        currentDir = path.dirname(currentDir);
    }

    return null;
}

export class DaemonClient implements vscode.Disposable {
    private clientProcess: any = null;
    private reconnectTimer: NodeJS.Timeout | null = null;
    private isDisposed = false;
    private readonly RECONNECT_INTERVAL_MS = 5000; // 5 seconds

    // Terminal registry: track active shell PIDs with MCP servers
    private activeTerminals: Set<number> = new Set();

    // Review feedback handling
    private pendingFeedbackResolvers: Map<string, (feedback: UserFeedback) => void> = new Map();
    private currentReviewId?: string;

    // General request-response handling
    private pendingRequestResolvers: Map<string, (response: any) => void> = new Map();

    private logger: StructuredLogger;

    constructor(
        private context: vscode.ExtensionContext,
        private outputChannel: vscode.OutputChannel,
        private syntheticPRProvider: SyntheticPRProvider,
        private walkthroughProvider: WalkthroughWebviewProvider
    ) {
        this.logger = new StructuredLogger(this.outputChannel);
    }

    start(): void {
        this.logger.info('Starting symposium client...');
        this.startClientProcess();
    }

    private async startClientProcess(): Promise<void> {
        if (this.isDisposed) return;

        this.logger.info(`Starting symposium-mcp client via shell`);

        // Spawn symposium-mcp client process
        const { spawn } = require('child_process');

        // Use shell to handle PATH resolution, same as macOS app
        this.clientProcess = spawn('/bin/sh', ['-c', 'symposium-mcp client'], {
            stdio: ['pipe', 'pipe', 'pipe'] // stdin, stdout, stderr
        });

        // Handle client process events
        this.clientProcess.on('spawn', () => {
            this.logger.info('✅ Symposium client process started');
            this.setupClientCommunication();
        });

        this.clientProcess.on('error', (error: Error) => {
            this.logger.error(`❌ Client process error: ${error.message}`);
            this.scheduleReconnect();
        });

        this.clientProcess.on('exit', (code: number | null) => {
            this.logger.info(`Client process exited with code: ${code}`);
            this.scheduleReconnect();
        });
    }

    private setupClientCommunication(): void {
        if (!this.clientProcess) return;

        // Set up stdout reader for daemon responses
        this.clientProcess.stdout.on('data', (data: Buffer) => {
            const text = data.toString();
            // Process each line as a potential JSON message
            const lines = text.split('\n');
            for (const line of lines) {
                if (line.trim()) {
                    try {
                        const message: IPCMessage = JSON.parse(line);
                        this.logger.debug(`Received message: ${message.type} (${message.id})`);
                        this.handleIncomingMessage(message).catch(error => {
                            this.logger.error(`Error handling message: ${error}`);
                        });
                    } catch (error) {
                        // Not JSON, might be daemon startup output - ignore
                    }
                }
            }
        });

        // Set up stderr reader for logging
        this.clientProcess.stderr.on('data', (data: Buffer) => {
            const stderrText = data.toString().trim();
            // If stderr already has structured format, use as-is, otherwise add CLIENT prefix
            if (stderrText.match(/^\[[A-Z-]+:\d+\]/)) {
                this.outputChannel.appendLine(stderrText);
            } else {
                this.logger.error(`Client stderr: ${stderrText}`);
            }
        });

        // Send initial Marco message to announce presence
        this.sendMarco();

        // Automatically register window if we're in a taskspace
        this.attemptAutoRegistration();
    }

    private async attemptAutoRegistration(): Promise<void> {
        try {
            const taskspaceUuid = await this.detectTaskspaceUUID();
            if (taskspaceUuid) {
                this.outputChannel.appendLine(`[WINDOW REG] Auto-registering window for taskspace: ${taskspaceUuid}`);
                await this.registerWindow(taskspaceUuid);
            } else {
                this.outputChannel.appendLine(`[WINDOW REG] Not in a taskspace, skipping auto-registration`);
            }
        } catch (error) {
            this.outputChannel.appendLine(`[WINDOW REG] Auto-registration failed: ${error}`);
        }
    }


    private async handleIncomingMessage(message: IPCMessage): Promise<void> {
        // First check: is this message for our window?
        // Marco messages (shellPid = 0) are broadcasts that everyone should ignore
        if (message.shellPid && !await this.isMessageForOurWindow(message.shellPid)) {
            return; // Silently ignore messages for other windows
        }

        // Forward compatibility: only process known message types
        if (message.type === 'present_walkthrough') {
            try {
                const walkthroughPayload = message.payload as PresentWalkthroughPayload;

                this.outputChannel.appendLine(`Received walkthrough with base_uri: ${walkthroughPayload.base_uri}`);
                this.outputChannel.appendLine(`Content length: ${walkthroughPayload.content.length} chars`);

                // Set base URI for file resolution
                if (walkthroughPayload.base_uri) {
                    this.walkthroughProvider.setBaseUri(walkthroughPayload.base_uri);
                }

                // Show walkthrough HTML content in webview
                this.walkthroughProvider.showWalkthroughHtml(walkthroughPayload.content);

                // Send success response back through daemon
                this.sendResponse(message.id, { success: true });
            } catch (error) {
                this.logger.error(`Error handling present_walkthrough: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'get_selection') {
            try {
                const selectionData = this.getCurrentSelection();
                this.sendResponse(message.id, {
                    success: true,
                    data: selectionData
                });
            } catch (error) {
                this.logger.error(`Error handling get_selection: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'log') {
            // Handle log messages - no response needed, just display in output channel
            try {
                const logPayload = message.payload as LogPayload;

                const levelPrefix = logPayload.level.toUpperCase();
                this.outputChannel.appendLine(`[${levelPrefix}] ${logPayload.message}`);
            } catch (error) {
                this.outputChannel.appendLine(`Error handling log message: ${error}`);
            }
        } else if (message.type === 'polo') {
            // Handle Polo messages - MCP server announcing presence
            try {
                this.outputChannel.appendLine(`[DISCOVERY] MCP server connected in terminal PID ${message.shellPid}`);

                // Add to terminal registry for Ask Socratic Shell integration
                this.activeTerminals.add(message.shellPid);
                this.outputChannel.appendLine(`[REGISTRY] Active terminals: [${Array.from(this.activeTerminals).join(', ')}]`);
            } catch (error) {
                this.outputChannel.appendLine(`Error handling polo message: ${error}`);
            }
        } else if (message.type === 'goodbye') {
            // Handle Goodbye messages - MCP server announcing departure
            try {
                this.outputChannel.appendLine(`[DISCOVERY] MCP server disconnected from terminal PID ${message.shellPid}`);

                // Remove from terminal registry for Ask Socratic Shell integration
                this.activeTerminals.delete(message.shellPid);
                this.outputChannel.appendLine(`[REGISTRY] Active terminals: [${Array.from(this.activeTerminals).join(', ')}]`);
            } catch (error) {
                this.outputChannel.appendLine(`Error handling goodbye message: ${error}`);
            }
        } else if (message.type === 'marco') {
            // Ignore Marco messages - these are broadcasts we send, MCP servers respond to them
            // Extensions don't need to respond to Marco broadcasts
        } else if (message.type === 'resolve_symbol_by_name') {
            // Handle symbol resolution requests from MCP server
            try {
                const symbolPayload = message.payload as ResolveSymbolPayload;

                this.outputChannel.appendLine(`[LSP] Resolving symbol: ${symbolPayload.name}`);

                // Call VSCode's LSP to find symbol definitions
                const symbols = await this.resolveSymbolByName(symbolPayload.name);

                this.sendResponse(message.id, {
                    success: true,
                    data: symbols
                });
            } catch (error) {
                this.outputChannel.appendLine(`Error handling resolve_symbol_by_name: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'find_all_references') {
            // Handle find references requests from MCP server
            try {
                const referencesPayload = message.payload as FindReferencesPayload;

                this.outputChannel.appendLine(`[LSP] Finding references for symbol: ${referencesPayload.symbol.name}`);

                // Call VSCode's LSP to find all references
                const references = await this.findAllReferences(referencesPayload.symbol);

                this.sendResponse(message.id, {
                    success: true,
                    data: references
                });
            } catch (error) {
                this.outputChannel.appendLine(`Error handling find_all_references: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'create_synthetic_pr') {
            // Handle synthetic PR creation
            const startTime = Date.now();
            this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Received create_synthetic_pr message`);
            try {
                const prPayload = message.payload as SyntheticPRPayload;
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Creating PR: ${prPayload.title}`);

                // Create PR UI using SyntheticPRProvider
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Calling syntheticPRProvider.createSyntheticPR`);
                await this.syntheticPRProvider.createSyntheticPR(prPayload);
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: syntheticPRProvider.createSyntheticPR completed`);

                // Collect user feedback
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Collecting user feedback`);
                const userFeedback = await this.collectUserFeedback(prPayload.review_id);
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: User feedback collected`);

                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Sending feedback response`);
                this.sendResponse(message.id, { success: true, data: userFeedback });
                this.outputChannel.appendLine(`[SYNTHETIC PR] ${Date.now() - startTime}ms: Feedback response sent`);
            } catch (error) {
                this.outputChannel.appendLine(`Error handling create_synthetic_pr: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'update_synthetic_pr') {
            // Handle synthetic PR updates
            try {
                const prPayload = message.payload as SyntheticPRPayload;
                this.outputChannel.appendLine(`[SYNTHETIC PR] Updating PR: ${prPayload.review_id}`);

                // Update PR UI using SyntheticPRProvider
                await this.syntheticPRProvider.updateSyntheticPR(prPayload);

                // Collect user feedback
                const userFeedback = await this.collectUserFeedback(prPayload.review_id);

                this.sendResponse(message.id, { success: true, data: userFeedback });
            } catch (error) {
                this.outputChannel.appendLine(`Error handling update_synthetic_pr: ${error}`);
                this.sendResponse(message.id, {
                    success: false,
                    error: error instanceof Error ? error.message : String(error)
                });
            }
        } else if (message.type === 'log') {
            // Handle log messages from daemon/MCP servers with structured formatting
            try {
                const logPayload = message.payload as { level: string; message: string };
                // The message already has structured prefix from Rust side, display as-is
                this.outputChannel.appendLine(logPayload.message);
            } catch (error) {
                this.outputChannel.appendLine(`Error handling log message: ${error}`);
            }
        } else if (message.type === 'reload_window') {
            // Handle reload window signal from daemon (on shutdown)
            this.logger.info('Received reload_window signal from daemon, reloading window...');
            vscode.commands.executeCommand('workbench.action.reloadWindow');
        } else if (message.type === 'response') {
            // Handle responses to our requests
            const resolver = this.pendingRequestResolvers.get(message.id);
            if (resolver) {
                this.pendingRequestResolvers.delete(message.id);
                const responsePayload = message.payload as ResponsePayload;
                if (responsePayload.success) {
                    resolver(responsePayload.data);
                } else {
                    this.logger.error(`Request ${message.id} failed: ${responsePayload.error}`);
                    resolver(null);
                }
            }
        } else if (message.type === 'taskspace_roll_call') {
            // Handle taskspace roll call - check if this is our taskspace and register window
            try {
                const rollCallPayload = message.payload as TaskspaceRollCallPayload;
                this.outputChannel.appendLine(`[WINDOW REG] Received roll call for taskspace: ${rollCallPayload.taskspace_uuid}`);

                // Check if this roll call is for our taskspace
                const currentTaskspaceUuid = await this.detectTaskspaceUUID();
                if (currentTaskspaceUuid === rollCallPayload.taskspace_uuid) {
                    this.outputChannel.appendLine(`[WINDOW REG] Roll call matches our taskspace, registering window`);
                    await this.registerWindow(rollCallPayload.taskspace_uuid);
                } else {
                    this.outputChannel.appendLine(`[WINDOW REG] Roll call not for us (ours: ${currentTaskspaceUuid}, theirs: ${rollCallPayload.taskspace_uuid})`);
                }
            } catch (error) {
                this.outputChannel.appendLine(`Error handling taskspace_roll_call: ${error}`);
            }
        } else {
            // Forward compatibility: silently ignore unknown message types for our window
            // Only log if this was actually meant for us (not a broadcast)
        }
    }

    private extractShellPidFromMessage(message: IPCMessage): number | null {
        return message.shellPid || null;
    }

    private async isMessageForOurWindow(shellPid: number): Promise<boolean> {
        try {
            // Get all terminal PIDs in the current VSCode window
            const terminals = vscode.window.terminals;

            for (const terminal of terminals) {
                try {
                    const terminalPid = await terminal.processId;
                    if (terminalPid === shellPid) {
                        this.outputChannel.appendLine(`Debug: shell PID ${shellPid} is in our window`);
                        return true;
                    }
                } catch (error) {
                    // Some terminals might not have accessible PIDs, skip them
                    continue;
                }
            }

            this.outputChannel.appendLine(`Debug: shell PID ${shellPid} is not in our window`);
            return false;
        } catch (error) {
            this.outputChannel.appendLine(`Error checking if message is for our window: ${error}`);
            // On error, default to processing the message (fail open)
            return true;
        }
    }

    private getCurrentSelection(): any {
        const activeEditor = vscode.window.activeTextEditor;

        if (!activeEditor) {
            return {
                selectedText: null,
                message: 'No active editor found'
            };
        }

        const selection = activeEditor.selection;

        if (selection.isEmpty) {
            return {
                selectedText: null,
                filePath: activeEditor.document.fileName,
                documentLanguage: activeEditor.document.languageId,
                isUntitled: activeEditor.document.isUntitled,
                message: 'No text selected in active editor'
            };
        }

        const selectedText = activeEditor.document.getText(selection);
        const startLine = selection.start.line + 1; // Convert to 1-based
        const startColumn = selection.start.character + 1; // Convert to 1-based
        const endLine = selection.end.line + 1;
        const endColumn = selection.end.character + 1;

        return {
            selectedText,
            filePath: activeEditor.document.fileName,
            startLine,
            startColumn,
            endLine,
            endColumn,
            lineNumber: startLine === endLine ? startLine : undefined,
            documentLanguage: activeEditor.document.languageId,
            isUntitled: activeEditor.document.isUntitled,
            message: `Selected ${selectedText.length} characters from ${startLine === endLine ? `line ${startLine}, columns ${startColumn}-${endColumn}` : `lines ${startLine}:${startColumn} to ${endLine}:${endColumn}`}`
        };
    }

    /**
     * Handle comment feedback from diff view
     */
    public handleCommentFeedback(comment: string, filePath: string, lineNumber: number): void {
        const reviewId = this.currentReviewId;
        if (!reviewId) {
            vscode.window.showErrorMessage('No active review found');
            return;
        }

        const resolver = this.pendingFeedbackResolvers.get(reviewId);
        if (!resolver) {
            vscode.window.showErrorMessage('No pending feedback request found');
            return;
        }

        // Resolve with comment feedback
        resolver({
            feedback_type: 'comment',
            review_id: reviewId,
            comment_text: comment,
            file_path: filePath,
            line_number: lineNumber
        });

        // Clear tree view and cleanup
        this.syntheticPRProvider.clearPR();
        this.pendingFeedbackResolvers.delete(reviewId);
    }

    /**
     * Handle review action from tree view button click
     */
    public handleReviewAction(action: string): void {
        const reviewId = this.currentReviewId;
        if (!reviewId) {
            vscode.window.showErrorMessage('No active review found');
            return;
        }

        const resolver = this.pendingFeedbackResolvers.get(reviewId);
        if (!resolver) {
            vscode.window.showErrorMessage('No pending feedback request found');
            return;
        }

        this.handleSpecificAction(action, reviewId, resolver);
    }

    private async handleSpecificAction(action: string, reviewId: string, resolver: (feedback: UserFeedback) => void): Promise<void> {
        if (action === 'comment') {
            const commentText = await vscode.window.showInputBox({
                prompt: 'Enter your comment',
                placeHolder: 'Type your comment here...',
                ignoreFocusOut: true
            });

            resolver({
                feedback_type: 'comment',
                review_id: reviewId,
                comment_text: commentText || '',
                file_path: 'review',
                line_number: 1
            });
        } else if (action === 'request_changes' || action === 'checkpoint') {
            const additionalNotes = await vscode.window.showInputBox({
                prompt: 'Any additional notes? (optional)',
                placeHolder: 'Additional instructions or context...',
                ignoreFocusOut: true
            });

            resolver({
                feedback_type: 'complete_review',
                review_id: reviewId,
                completion_action: action as 'request_changes' | 'checkpoint',
                additional_notes: additionalNotes
            });
        } else {
            resolver({
                feedback_type: 'complete_review',
                review_id: reviewId,
                completion_action: 'return'
            });
        }

        // Clear tree view after action
        this.syntheticPRProvider.clearPR();
        this.pendingFeedbackResolvers.delete(reviewId);
    }

    /**
     * Collect user feedback for a review
     * This method blocks until the user provides feedback via tree view buttons
     */
    private async collectUserFeedback(reviewId: string): Promise<UserFeedback> {
        this.currentReviewId = reviewId;

        // Automatically show the review
        vscode.commands.executeCommand('socratic-shell.showReview');

        return new Promise<UserFeedback>((resolve) => {
            this.pendingFeedbackResolvers.set(reviewId, resolve);
        });
    }

    private sendResponse(messageId: string, response: ResponsePayload): void {
        if (!this.clientProcess || this.clientProcess.killed) {
            this.outputChannel.appendLine(`Cannot send response - client process not available`);
            return;
        }

        const responseMessage: IPCMessage = {
            type: 'response',
            payload: response,
            id: messageId,
            shellPid: 0,
        };

        try {
            this.clientProcess.stdin.write(JSON.stringify(responseMessage) + '\n');
        } catch (error) {
            this.outputChannel.appendLine(`Failed to send response: ${error}`);
        }
    }

    private async detectTaskspaceUUID(): Promise<string | null> {
        return getCurrentTaskspaceUuid();
    }

    private async registerWindow(taskspaceUuid: string): Promise<void> {
        try {
            // Generate unique window identifier
            const windowUUID = crypto.randomUUID();

            // Get current window title
            const config = vscode.workspace.getConfiguration();
            const originalTitle = config.get<string>('window.title') || '';

            // Set temporary title with unique identifier
            const uniqueIdentifier = `[SYMPOSIUM:${windowUUID}]`;
            const tempTitle = `${uniqueIdentifier} ${originalTitle}`;
            await config.update('window.title', tempTitle, vscode.ConfigurationTarget.Workspace);

            this.outputChannel.appendLine(`[WINDOW REG] Set temporary title: ${tempTitle}`);

            // Send registration message to Swift app using existing helper
            const payload: RegisterTaskspaceWindowPayload = {
                window_title: uniqueIdentifier,  // Send just the unique part for substring matching
                taskspace_uuid: taskspaceUuid
            };

            // Use existing sendRequest helper with timeout
            const response = await this.sendRequest<{ success: boolean }>('register_taskspace_window', payload, 5000);

            if (response?.success) {
                this.outputChannel.appendLine(`[WINDOW REG] Successfully registered window for taskspace: ${taskspaceUuid}`);
            } else {
                this.outputChannel.appendLine(`[WINDOW REG] Failed to register window for taskspace: ${taskspaceUuid}`);
            }

            // Restore original title
            await config.update('window.title', originalTitle, vscode.ConfigurationTarget.Workspace);
            this.outputChannel.appendLine(`[WINDOW REG] Restored original title`);

        } catch (error) {
            this.outputChannel.appendLine(`[WINDOW REG] Error during window registration: ${error}`);

            // Ensure title is restored even on error
            try {
                const config = vscode.workspace.getConfiguration();
                const originalTitle = config.get<string>('window.title') || '';
                if (originalTitle.includes('[SYMPOSIUM:')) {
                    // Extract original title from temporary title
                    const match = originalTitle.match(/^\[SYMPOSIUM:[^\]]+\] (.*)$/);
                    if (match) {
                        await config.update('window.title', match[1], vscode.ConfigurationTarget.Workspace);
                    }
                }
            } catch (restoreError) {
                this.outputChannel.appendLine(`[WINDOW REG] Error restoring title: ${restoreError}`);
            }
        }
    }

    /**
     * Send a reference to the active AI terminal via IPC
     */
    public async sendReferenceToActiveShell(key: string, value: any): Promise<void> {
        const terminals = vscode.window.terminals;
        if (terminals.length === 0) {
            vscode.window.showWarningMessage('No terminals found. Please open a terminal with an active AI assistant.');
            return;
        }

        // Get active terminals with MCP servers from registry
        const activeTerminals = this.getActiveTerminals();
        this.outputChannel.appendLine(`Active MCP server terminals: [${Array.from(activeTerminals).join(', ')}]`);

        if (activeTerminals.size === 0) {
            vscode.window.showWarningMessage('No terminals with active MCP servers found. Please ensure you have a terminal with an active AI assistant (like Q chat or Claude CLI) running.');
            return;
        }

        // Filter terminals to only those with active MCP servers
        const terminalChecks = await Promise.all(
            terminals.map(async (terminal) => {
                const shellPID = await terminal.processId;
                const isAiEnabled = shellPID && activeTerminals.has(shellPID);
                return { terminal, shellPID, isAiEnabled };
            })
        );

        const aiEnabledTerminals = terminalChecks
            .filter(check => check.isAiEnabled)
            .map(check => ({ terminal: check.terminal, shellPID: check.shellPID }));

        if (aiEnabledTerminals.length === 0) {
            vscode.window.showWarningMessage('No AI-enabled terminals found. Please ensure you have a terminal with an active MCP server running.');
            return;
        }

        // Simple case - exactly one AI-enabled terminal
        if (aiEnabledTerminals.length === 1) {
            const { shellPID } = aiEnabledTerminals[0];
            if (shellPID) {
                this.sendStoreReferenceToShell(shellPID, key, value);
                this.outputChannel.appendLine(`Reference ${key} sent to shell ${shellPID}`);
            }
            return;
        }

        // Multiple terminals - send to all (or we could show a picker)
        for (const { shellPID } of aiEnabledTerminals) {
            if (shellPID) {
                this.sendStoreReferenceToShell(shellPID, key, value);
                this.outputChannel.appendLine(`Reference ${key} sent to shell ${shellPID}`);
            }
        }
    }

    public sendStoreReferenceToShell(shellPid: number, key: string, value: any): void {
        if (!this.clientProcess || this.clientProcess.stdin?.destroyed) {
            this.outputChannel.appendLine(`Cannot send store_reference - client not connected`);
            return;
        }

        const storePayload: StoreReferencePayload = {
            key,
            value
        };

        const storeMessage: IPCMessage = {
            shellPid,
            type: 'store_reference',
            payload: storePayload,
            id: crypto.randomUUID()
        };

        try {
            this.clientProcess.stdin?.write(JSON.stringify(storeMessage) + '\n');
            this.outputChannel.appendLine(`[REFERENCE] Stored reference ${key} for shell ${shellPid}`);
        } catch (error) {
            this.outputChannel.appendLine(`Failed to send store_reference to shell ${shellPid}: ${error}`);
        }
    }


    private sendMarco(): void {
        if (!this.clientProcess || this.clientProcess.killed) {
            this.outputChannel.appendLine(`Cannot send Marco - client process not available`);
            return;
        }

        const marcoMessage = {
            type: 'marco',
            payload: {},
            id: crypto.randomUUID()
        };

        try {
            this.clientProcess.stdin.write(JSON.stringify(marcoMessage) + '\n');
            this.logger.info('[DISCOVERY] Sent Marco broadcast to discover MCP servers');
        } catch (error) {
            this.outputChannel.appendLine(`Failed to send Marco: ${error}`);
        }
    }


    private async tryStartDaemon(): Promise<void> {
        // With the new client architecture, we don't need to manage daemons directly
        // The client mode handles daemon startup automatically
        this.outputChannel.appendLine('✅ Using client mode - daemon management handled automatically');
        return Promise.resolve();
    }

    private scheduleReconnect(): void {
        if (this.isDisposed) return;

        this.clearReconnectTimer();
        this.reconnectTimer = setTimeout(() => {
            this.startClientProcess();
        }, this.RECONNECT_INTERVAL_MS);
    }

    private clearReconnectTimer(): void {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
    }

    /**
     * Resolve symbol by name using VSCode's LSP
     */
    private async resolveSymbolByName(symbolName: string): Promise<SymbolDef[]> {
        try {
            // Get all workspace symbols matching the name
            const symbols = await vscode.commands.executeCommand<vscode.SymbolInformation[]>(
                'vscode.executeWorkspaceSymbolProvider',
                symbolName
            );

            if (!symbols || symbols.length === 0) {
                return [];
            }

            // Convert VSCode symbols to our format
            const resolvedSymbols: SymbolDef[] = symbols.map(symbol => this.vscodeSymbolToSymbolDef(symbol));

            return resolvedSymbols;
        } catch (error) {
            this.outputChannel.appendLine(`Error in resolveSymbolByName: ${error}`);
            throw error;
        }
    }

    private vscodeSymbolToSymbolDef(symbol: vscode.SymbolInformation): SymbolDef {
        let definedAt = symbol.location
        let result: SymbolDef = {
            name: symbol.name,
            definedAt: this.vscodeLocationToRange(symbol.location),
        };

        switch (symbol.kind) {
            case vscode.SymbolKind.File: result.kind = "File"; break;
            case vscode.SymbolKind.Module: result.kind = "Module"; break;
            case vscode.SymbolKind.Namespace: result.kind = "Namespace"; break;
            case vscode.SymbolKind.Package: result.kind = "Package"; break;
            case vscode.SymbolKind.Class: result.kind = "Class"; break;
            case vscode.SymbolKind.Method: result.kind = "Method"; break;
            case vscode.SymbolKind.Property: result.kind = "Property"; break;
            case vscode.SymbolKind.Field: result.kind = "Field"; break;
            case vscode.SymbolKind.Constructor: result.kind = "Constructor"; break;
            case vscode.SymbolKind.Enum: result.kind = "Enum"; break;
            case vscode.SymbolKind.Interface: result.kind = "Interface"; break;
            case vscode.SymbolKind.Function: result.kind = "Function"; break;
            case vscode.SymbolKind.Variable: result.kind = "Variable"; break;
            case vscode.SymbolKind.Constant: result.kind = "Constant"; break;
            case vscode.SymbolKind.String: result.kind = "String"; break;
            case vscode.SymbolKind.Number: result.kind = "Number"; break;
            case vscode.SymbolKind.Boolean: result.kind = "Boolean"; break;
            case vscode.SymbolKind.Array: result.kind = "Array"; break;
            case vscode.SymbolKind.Object: result.kind = "Object"; break;
            case vscode.SymbolKind.Key: result.kind = "Key"; break;
            case vscode.SymbolKind.Null: result.kind = "Null"; break;
            case vscode.SymbolKind.EnumMember: result.kind = "EnumMember"; break;
            case vscode.SymbolKind.Struct: result.kind = "Struct"; break;
            case vscode.SymbolKind.Event: result.kind = "Event"; break;
            case vscode.SymbolKind.Operator: result.kind = "Operator"; break;
            case vscode.SymbolKind.TypeParameter: result.kind = "TypeParameter"; break;
        }

        return result;
    }

    private vscodeLocationToRange(location: vscode.Location): FileRange {
        return {
            path: location.uri.fsPath,
            start: {
                line: location.range.start.line + 1,
                column: location.range.start.character + 1,
            },
            end: {
                line: location.range.end.line + 1,
                column: location.range.end.character + 1,
            },
        };
    }


    /**
     * Find all references to a symbol using VSCode's LSP
     */
    private async findAllReferences(symbol: SymbolDef): Promise<FileRange[]> {
        try {
            // Convert relative path back to URI
            const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
            if (!workspaceFolder) {
                throw new Error('No workspace folder found');
            }

            // Find all references using LSP
            this.outputChannel.appendLine(`workspaceFolder.uri: ${workspaceFolder.uri}`);
            this.outputChannel.appendLine(`symbol.definedAt.path: ${symbol.definedAt.path}`);
            const locations = await vscode.commands.executeCommand<vscode.Location[]>(
                'vscode.executeReferenceProvider',
                vscode.Uri.file(path.isAbsolute(symbol.definedAt.path)
                    ? symbol.definedAt.path
                    : path.resolve(workspaceFolder.uri.fsPath, symbol.definedAt.path)),
                new vscode.Position(symbol.definedAt.start.line - 1, symbol.definedAt.start.column - 1)
            );

            return locations.map(location => this.vscodeLocationToRange(location));
        } catch (error) {
            this.outputChannel.appendLine(`Error in findAllReferences: ${error}`);
            throw error;
        }
    }

    /**
     * Send an IPC request and wait for response
     */
    async sendRequest<T>(type: string, payload: any, timeoutMs: number = 5000): Promise<T | null> {
        try {
            const messageId = crypto.randomUUID();
            const message: IPCMessage = {
                shellPid: process.pid,
                type: type,
                payload: payload,
                id: messageId
            };

            // Send the message
            if (!this.clientProcess || !this.clientProcess.stdin) {
                throw new Error('Daemon client not connected');
            }

            this.clientProcess.stdin.write(JSON.stringify(message) + '\n');
            this.logger.info(`Sent ${type} request with ID: ${messageId}`);

            // Wait for response
            return new Promise<T | null>((resolve) => {
                this.pendingRequestResolvers.set(messageId, resolve);

                // Timeout after specified time
                setTimeout(() => {
                    if (this.pendingRequestResolvers.has(messageId)) {
                        this.pendingRequestResolvers.delete(messageId);
                        this.logger.error(`Request ${messageId} timed out after ${timeoutMs}ms`);
                        resolve(null);
                    }
                }, timeoutMs);
            });

        } catch (error) {
            this.logger.error(`Error sending ${type} request: ${error}`);
            return null;
        }
    }

    dispose(): void {
        this.isDisposed = true;
        this.clearReconnectTimer();

        if (this.clientProcess && !this.clientProcess.killed) {
            this.clientProcess.kill();
            this.clientProcess = null;
        }

        this.outputChannel.appendLine('Symposium client disposed');
    }

    /**
     * Get the set of active terminal shell PIDs with MCP servers
     * For Ask Socratic Shell integration
     */
    getActiveTerminals(): Set<number> {
        return new Set(this.activeTerminals); // Return a copy to prevent external modification
    }
} 