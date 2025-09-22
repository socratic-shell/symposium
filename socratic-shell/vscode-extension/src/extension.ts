import * as vscode from 'vscode';
import * as net from 'net';
import * as path from 'path';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { quote } from 'shell-quote';

import { WalkthroughWebviewProvider } from './walkthroughWebview';
import { Bus } from './bus';
import { DaemonClient } from './ipc';
import { StructuredLogger } from './structuredLogger';
import { getCurrentTaskspaceUuid } from './taskspaceUtils';
import { debugLog } from './logging';

// Global logger instance for the extension
let globalLogger: StructuredLogger | null = null;

/**
 * Get the global logger instance (available after activation)
 */
export function getLogger(): StructuredLogger | null {
    return globalLogger;
}

// TEST TEST TEST 


// 💡: Types for IPC communication with MCP server

interface LogPayload {
    level: 'info' | 'error' | 'debug';
    message: string;
}

interface GetSelectionPayload {
    // Empty payload
}

interface PoloPayload {
    // Shell PID now at top level
}

interface GoodbyePayload {
    // Shell PID now at top level  
}

interface ResolveSymbolPayload {
    name: string;
}

interface FindReferencesPayload {
    symbol: SymbolDef;
}

interface ResponsePayload {
    success: boolean;
    error?: string;
    data?: any;
}

/**
 * Request for taskspace state operations (unified get/update)
 */
interface TaskspaceStateRequest {
    /** Path to .symposium project directory */
    project_path: string;
    /** UUID of the taskspace */
    taskspace_uuid: string;
    /** New name to set (null = don't update) */
    name: string | null;
    /** New description to set (null = don't update) */
    description: string | null;
}

/**
 * Response from Symposium app when querying taskspace state
 * Contains taskspace metadata for agent initialization
 */
interface TaskspaceStateResponse {
    /** User-visible taskspace name */
    name?: string;
    /** User-visible taskspace description */
    description?: string;
    /** LLM task description (present only during initial agent startup) */
    initial_prompt?: string;
    /** Command to launch the appropriate agent */
    agent_command: string[];
}

// ANCHOR: store_reference_payload
interface StoreReferencePayload {
    /** UUID key for the reference */
    key: string;
    /** Arbitrary JSON value - self-documenting structure determined by extension */
    value: any;
}
// ANCHOR_END: store_reference_payload

interface PresentWalkthroughPayload {
    content: string;  // HTML content with resolved XML elements
    baseUri: string;
}

// ANCHOR: taskspace_roll_call_payload
interface TaskspaceRollCallPayload {
    taskspace_uuid: string;
}
// ANCHOR_END: taskspace_roll_call_payload

// ANCHOR: register_taskspace_window_payload
interface RegisterTaskspaceWindowPayload {
    window_title: string;
    taskspace_uuid: string;
}
// ANCHOR_END: register_taskspace_window_payload

type WalkthroughElement =
    | string  // ResolvedMarkdownElement (now serialized as plain string)
    | { comment: ResolvedComment }
    | { files: FileChange[] }  // GitDiffElement - named field serializes as {"files": [...]}
    | { action: ResolvedAction };

interface ResolvedComment {
    locations: FileRange[];
    icon?: string;
    content: WalkthroughElement[];
}

interface ResolvedAction {
    button: string;
    tell_agent?: string;
}

interface FileLocation {
    line: number;
    column: number;
}

interface FileRange {
    path: string;
    start: FileLocation;
    end: FileLocation;
    content?: string;
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

interface UserFeedback {
    feedback_type: 'comment' | 'complete_review';
    review_id: string;
    // For Comment variant
    file_path?: string;
    line_number?: number;
    comment_text?: string;
    context_lines?: string[];
    // For CompleteReview variant
    completion_action?: 'request_changes' | 'checkpoint' | 'return';
    additional_notes?: string;
}

// 💡: Corresponds to `symposium_mcp::ide::SymbolRef` in the Rust code
interface SymbolDef {
    name: String,
    kind?: String,
    definedAt: FileRange,
}

// 💡: Corresponds to `symposium_mcp::ide::SymbolRef` in the Rust code
interface SymbolRef {
    definition: SymbolDef,
    referencedAt: FileLocation,
}

// 💡: Corresponds to `symposium_mcp::ide::FileRange` in the Rust code
interface FileRange {
    path: string,
    start: FileLocation,
    end: FileLocation,
    content?: string,
}

// 💡: Corresponds to `symposium_mcp::ide::FileLocation` in the Rust code
interface FileLocation {
    line: number,    // 💡: 1-based, vscode is 0-based
    column: number,  // 💡: 1-based, vscode is 0-based
}

// 💡: Check if VSCode is running in a taskspace environment and auto-launch agent
async function checkTaskspaceEnvironment(bus: Bus): Promise<void> {
    debugLog('Checking for taskspace environment...');

    const taskspaceUuid = getCurrentTaskspaceUuid();
    if (!taskspaceUuid) {
        debugLog('Not in a taskspace environment');
        return;
    }

    debugLog(`✅ Taskspace detected! UUID: ${taskspaceUuid}`);

    // Send taskspace_state message to get current taskspace information
    const payload: TaskspaceStateRequest = {
        project_path: getProjectPath(),
        taskspace_uuid: taskspaceUuid,
        name: null,        // Read-only operation
        description: null  // Read-only operation
    };
    const response = await bus.daemonClient.sendRequest<TaskspaceStateResponse>('taskspace_state', payload);
    debugLog(`App responded with ${JSON.stringify(response)}`);

    if (response) {
        debugLog(`Taskspace: ${response.name || 'Unnamed'} - ${response.description || 'No description'}`);
        if (response.initial_prompt) {
            debugLog('Initial prompt available - launching agent for first-time setup');
        } else {
            debugLog('Resuming existing taskspace');
        }
        debugLog(`Launching agent: ${response.agent_command.join(' ')}`);
        await launchAIAgent(bus, response.agent_command, taskspaceUuid);
    } else {
        debugLog('No taskspace state received from app');
    }
}

// 💡: Launch AI agent in terminal with provided command
async function launchAIAgent(bus: Bus, agentCommand: string[], taskspaceUuid: string): Promise<void> {
    try {
        debugLog(`Launching agent with command: ${agentCommand.join(' ')}`);

        // Create new terminal for the agent
        const terminal = vscode.window.createTerminal({
            name: `Symposium`,
            cwd: vscode.workspace.workspaceFolders?.[0].uri.fsPath
        });

        // Show the terminal
        terminal.show();

        // Send the agent command - use shell-quote for proper escaping
        const quotedCommand = quote(agentCommand);
        terminal.sendText(quotedCommand);

        debugLog('Agent launched successfully');

    } catch (error) {
        debugLog(`Error launching AI agent: ${error}`);
    }
}

export function activate(context: vscode.ExtensionContext) {

    // 💡: Create dedicated output channel for cleaner logging
    const outputChannel = vscode.window.createOutputChannel('Symposium');
    
    // Create global logger for the extension
    const logger = new StructuredLogger(outputChannel, 'EXTENSION');
    globalLogger = logger; // Set global reference
    logger.info('Symposium extension is now active');
    console.log('Symposium extension is now active');

    // Create the central bus
    const bus = new Bus(context, logger);

    // 💡: PID Discovery Testing - Log VSCode and terminal PIDs
    logPIDDiscovery().catch(error => {
        debugLog(`Error in PID discovery: ${error}`);
    });

    // Create walkthrough webview provider
    const walkthroughProvider = new WalkthroughWebviewProvider(context.extensionUri, bus);
    bus.setWalkthroughProvider(walkthroughProvider);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(WalkthroughWebviewProvider.viewType, walkthroughProvider)
    );

    // Register walkthrough comment reply command (legacy - may not be needed)
    const walkthroughCommentCommand = vscode.commands.registerCommand('symposium.addWalkthroughComment',
        (reply: vscode.CommentReply) => walkthroughProvider.handleCommentSubmission(reply)
    );
    context.subscriptions.push(walkthroughCommentCommand);

    // Register new walkthrough comment reply command that uses symposium-ref
    const walkthroughReplyCommand = vscode.commands.registerCommand('symposium.replyToWalkthroughComment',
        async (commentData: { file: string; range: { start: { line: number }; end: { line: number } }; comment: string }) => {
            try {
                console.log('Walkthrough reply command called with data:', commentData);

                // Create reference data for symposium-ref
                const referenceData = {
                    'in-reply-to-comment-at': {
                        file: vscode.workspace.asRelativePath(commentData.file),
                        start: commentData.range.start,
                        end: commentData.range.end,
                        comment: commentData.comment
                    }
                };
                console.log('Reference data created:', referenceData);

                // Use sendToActiveTerminal to handle both storing data and inserting XML
                console.log('Sending reference to active terminal...');
                await bus.sendToActiveTerminal(referenceData, { includeNewline: false });
                console.log('Reference sent successfully');
            } catch (error) {
                console.error('Failed to reply to walkthrough comment:', error);
                vscode.window.showErrorMessage(`Failed to reply to comment: ${error}`);
            }
        }
    );
    context.subscriptions.push(walkthroughReplyCommand);

    console.log('Webview provider created successfully');

    // 💡: Set up daemon client connection for message bus communication
    const daemonClient = new DaemonClient(context, walkthroughProvider, logger);
    bus.setDaemonClient(daemonClient);
    
    // Set daemon client on global logger for unified logging
    logger.setDaemonClient(daemonClient);

    daemonClient.start();

    // 💡: Check for taskspace environment and auto-launch agent if needed
    // (Must be after DaemonClient is initialized)
    checkTaskspaceEnvironment(bus).catch(error => {
        debugLog(`Error in taskspace detection: ${error}`);
    });

    // 💡: Set up universal selection detection for interactive code review
    setupSelectionDetection(bus);

    // Register review action command for tree view buttons
    // 💡: Show review command - displays the walkthrough panel
    const showReviewCommand = vscode.commands.registerCommand('symposium.showReview', () => {
        // Focus on the walkthrough webview panel
        vscode.commands.executeCommand('symposium.walkthrough.focus');
    });

    const reviewActionCommand = vscode.commands.registerCommand('symposium.reviewAction', (action: string) => {
        daemonClient.handleReviewAction(action);
    });

    // 💡: Copy review command is now handled via webview postMessage
    const copyReviewCommand = vscode.commands.registerCommand('symposium.copyReview', () => {
        vscode.window.showInformationMessage('Use the Copy Review button in the review panel');
    });

    // 💡: PID discovery command for testing
    const logPIDsCommand = vscode.commands.registerCommand('symposium.logPIDs', async () => {
        outputChannel.show(); // Bring output channel into focus
        await logPIDDiscovery();
        vscode.window.showInformationMessage('PID information logged to Symposium output channel');
    });

    // Window title toggle command for POC
    const toggleWindowTitleCommand = vscode.commands.registerCommand('symposium.toggleWindowTitle', async () => {
        const config = vscode.workspace.getConfiguration();
        const currentTitle = config.get<string>('window.title') || '';

        if (currentTitle.startsWith('[symposium] ')) {
            // Remove the prefix
            const newTitle = currentTitle.replace('[symposium] ', '');
            await config.update('window.title', newTitle, vscode.ConfigurationTarget.Workspace);
            vscode.window.showInformationMessage('Removed [symposium] from window title');
        } else {
            // Add the prefix
            const newTitle = `[symposium] ${currentTitle}`;
            await config.update('window.title', newTitle, vscode.ConfigurationTarget.Workspace);
            vscode.window.showInformationMessage('Added [symposium] to window title');
        }
    });

    context.subscriptions.push(showReviewCommand, reviewActionCommand, copyReviewCommand, logPIDsCommand, daemonClient, toggleWindowTitleCommand);

    // Return API for Discuss in Symposium integration
    return {
        discoverActiveShells: () => daemonClient.discoverActiveShells()
    };
}

// 💡: Set up universal selection detection for interactive code review
function setupSelectionDetection(bus: Bus): void {
    const { context } = bus;

    debugLog('Setting up universal selection detection...');

    // 💡: Track current selection state
    let currentSelection: {
        editor: vscode.TextEditor;
        selection: vscode.Selection;
    } | null = null;

    // 💡: Listen for selection changes to track current selection
    const selectionListener = vscode.window.onDidChangeTextEditorSelection((event) => {
        if (event.selections.length > 0 && !event.selections[0].isEmpty) {
            const selection = event.selections[0];

            // Store current selection state
            currentSelection = {
                editor: event.textEditor,
                selection: selection
            };
        } else {
            currentSelection = null;
        }
    });

    // 💡: Register Code Action Provider for "Symposium" section
    const codeActionProvider = vscode.languages.registerCodeActionsProvider(
        '*', // All file types
        {
            provideCodeActions(document, range, context) {
                // Only show when there's a non-empty selection
                if (!range.isEmpty) {
                    const action = new vscode.CodeAction(
                        'Discuss in Symposium',
                        vscode.CodeActionKind.QuickFix
                    );
                    action.command = {
                        command: 'symposium.chatAboutSelection',
                        title: 'Discuss in Symposium'
                    };
                    action.isPreferred = true; // Show at top of list

                    return [action];
                }
                return [];
            }
        },
        {
            providedCodeActionKinds: [vscode.CodeActionKind.QuickFix]
        }
    );

    // 💡: Register command for when user clicks the code action
    const chatIconCommand = vscode.commands.registerCommand('symposium.chatAboutSelection', async () => {
        if (currentSelection) {
            const selectedText = currentSelection.editor.document.getText(currentSelection.selection);
            const filePath = currentSelection.editor.document.fileName;
            const startLine = currentSelection.selection.start.line + 1;
            const startColumn = currentSelection.selection.start.character + 1;
            const endLine = currentSelection.selection.end.line + 1;
            const endColumn = currentSelection.selection.end.character + 1;

            debugLog(`CHAT ICON CLICKED!`);
            debugLog(`Selected: "${selectedText}"`);
            debugLog(`Location: ${filePath}:${startLine}:${startColumn}-${endLine}:${endColumn}`);

            // Use new consolidated sendToActiveTerminal method
            try {
                const relativePath = vscode.workspace.asRelativePath(filePath);
                const referenceData = {
                    relativePath: relativePath,
                    selectionRange: {
                        start: { line: startLine, column: startColumn },
                        end: { line: endLine, column: endColumn }
                    },
                    selectedText: selectedText,
                };

                await bus.sendToActiveTerminal(referenceData, { includeNewline: false });
                debugLog(`Compact reference sent for ${relativePath}:${startLine}`);
            } catch (error) {
                debugLog(`Failed to send reference: ${error}`);
                vscode.window.showErrorMessage('Failed to send reference to terminal');
            }
        } else {
            debugLog('Chat action triggered but no current selection found');
        }
    });

    context.subscriptions.push(selectionListener, codeActionProvider, chatIconCommand);
    debugLog('Selection detection with Code Actions setup complete');
}

/**
 * Get the project path (.symposium directory) for the current workspace
 * Returns project path if valid, empty string otherwise
 */
function getProjectPath(): string {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return '';
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath;

    // Walk up the directory tree looking for .symposium directory
    let currentDir = workspaceRoot;
    while (currentDir !== path.dirname(currentDir)) { // Stop at filesystem root
        const symposiumDir = path.join(currentDir, '.symposium');
        if (fs.existsSync(symposiumDir)) {
            return currentDir;
        }
        currentDir = path.dirname(currentDir);
    }

    return '';
}

// 💡: PID Discovery Testing - Log all relevant PIDs for debugging
async function logPIDDiscovery(): Promise<void> {
    debugLog('=== PID DISCOVERY TESTING ===');

    // Extension process info
    debugLog(`Extension process PID: ${process.pid}`);
    debugLog(`Extension parent PID: ${process.ppid}`);

    // Try to find VSCode PID by walking up the process tree
    const vscodePid = findVSCodePID();
    if (vscodePid) {
        debugLog(`Found VSCode PID: ${vscodePid}`);
    } else {
        debugLog('Could not find VSCode PID');
    }

    // Log terminal PIDs (handle the Promise properly)
    const terminals = vscode.window.terminals;
    debugLog(`Found ${terminals.length} terminals:`);

    for (let i = 0; i < terminals.length; i++) {
        const terminal = terminals[i];
        try {
            // terminal.processId returns a Promise in newer VSCode versions
            const pid = await terminal.processId;
            debugLog(`  Terminal ${i}: name="${terminal.name}", PID=${pid}`);
        } catch (error) {
            debugLog(`  Terminal ${i}: name="${terminal.name}", PID=<error: ${error}>`);
        }
    }

    // Set up terminal monitoring
    const terminalListener = vscode.window.onDidOpenTerminal(async (terminal) => {
        try {
            const pid = await terminal.processId;
            debugLog(`NEW TERMINAL: name="${terminal.name}", PID=${pid}`);
        } catch (error) {
            debugLog(`NEW TERMINAL: name="${terminal.name}", PID=<error: ${error}>`);
        }
    });

    debugLog('=== END PID DISCOVERY ===');
}

// 💡: Attempt to find VSCode PID by walking up process tree
function findVSCodePID(): number | null {
    const { execSync } = require('child_process');

    try {
        let currentPid = process.pid;

        // Walk up the process tree
        for (let i = 0; i < 10; i++) { // Safety limit
            try {
                // Get process info (works on macOS/Linux)
                const psOutput = execSync(`ps -p ${currentPid} -o pid,ppid,comm,args`, { encoding: 'utf8' });
                const lines = psOutput.trim().split('\n');

                if (lines.length < 2) break;

                const processLine = lines[1].trim();
                const parts = processLine.split(/\s+/);
                const pid = parseInt(parts[0]);
                const ppid = parseInt(parts[1]);
                const command = parts.slice(3).join(' '); // Full command line

                // Check if this looks like the main VSCode process (not helper processes)
                if ((command.includes('Visual Studio Code') || command.includes('Code.app') || command.includes('Electron'))
                    && !command.includes('Code Helper')) {
                    debugLog(`Found VSCode PID: ${pid}`);
                    return pid;
                }

                currentPid = ppid;
                if (ppid <= 1) break; // Reached init process

            } catch (error) {
                break;
            }
        }

        debugLog('Could not find VSCode PID in process tree');
        return null;

    } catch (error) {
        debugLog(`PID discovery error: ${error}`);
        return null;
    }
}

export function deactivate() { }