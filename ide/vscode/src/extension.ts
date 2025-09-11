import * as vscode from 'vscode';
import * as net from 'net';
import * as path from 'path';
import * as fs from 'fs';
import * as crypto from 'crypto';
import { quote } from 'shell-quote';
import { SyntheticPRProvider } from './syntheticPRProvider';
import { WalkthroughWebviewProvider } from './walkthroughWebview';
import { Bus } from './bus';
import { DaemonClient } from './ipc';
import { StructuredLogger } from './structuredLogger';

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

interface GetTaskspaceStatePayload {
    taskspaceUuid: string;
}

/**
 * Response from Symposium app when querying taskspace state
 * Used to determine if and how to launch an AI agent for a taskspace
 */
interface TaskspaceStateResponse {
    /** Command and arguments to execute in terminal (e.g., ['q', 'chat', '--resume']) */
    agentCommand: string[];
    /** Whether the agent should be launched (false for completed/unknown taskspaces) */
    shouldLaunch: boolean;
}

interface SyntheticPRPayload {
    review_id: string;
    title: string;
    description: any;
    commit_range: string;
    files_changed: FileChange[];
    comment_threads: CommentThread[];
    status: string;
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
    base_uri: string;
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

/**
 * Investigate current workspace to determine if we're in a taskspace
 * Returns taskspace UUID if valid, null otherwise
 */
export function getCurrentTaskspaceUuid(): string | null {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath;
    const taskUuidPattern = /^task-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i;

    // Walk up the directory tree looking for task-UUID directory with taskspace.json
    let currentDir = workspaceRoot;
    while (currentDir !== path.dirname(currentDir)) { // Stop at filesystem root
        const dirName = path.basename(currentDir);
        const match = dirName.match(taskUuidPattern);

        if (match) {
            const taskspaceJsonPath = path.join(currentDir, 'taskspace.json');
            if (fs.existsSync(taskspaceJsonPath)) {
                return match[1]; // Return the UUID
            }
        }

        currentDir = path.dirname(currentDir);
    }

    return null; // No taskspace found in directory tree
}

// 💡: Check if VSCode is running in a taskspace environment and auto-launch agent
async function checkTaskspaceEnvironment(outputChannel: vscode.OutputChannel, bus: Bus): Promise<void> {
    outputChannel.appendLine('Checking for taskspace environment...');

    const taskspaceUuid = getCurrentTaskspaceUuid();
    if (!taskspaceUuid) {
        outputChannel.appendLine('Not in a taskspace environment');
        return;
    }

    outputChannel.appendLine(`✅ Taskspace detected! UUID: ${taskspaceUuid}`);

    // Send get_taskspace_state message as documented in the flow
    const payload: GetTaskspaceStatePayload = { taskspaceUuid };
    const response = await bus.daemonClient.sendRequest<TaskspaceStateResponse>('get_taskspace_state', payload);
    outputChannel.appendLine(`App responded with ${JSON.stringify(response)}`);

    if (response && response.shouldLaunch) {
        outputChannel.appendLine(`Launching agent: ${response.agentCommand.join(' ')}`);
        await launchAIAgent(outputChannel, bus, response.agentCommand, taskspaceUuid);
    } else {
        outputChannel.appendLine('App indicated agent should not be launched');
    }
}

// 💡: Launch AI agent in terminal with provided command
async function launchAIAgent(outputChannel: vscode.OutputChannel, bus: Bus, agentCommand: string[], taskspaceUuid: string): Promise<void> {
    try {
        // Use shell-quote library for proper escaping
        const escapedCommand = quote(agentCommand);

        outputChannel.appendLine(`Launching agent with command: ${escapedCommand}`);

        // Create new terminal for the agent
        const terminal = vscode.window.createTerminal({
            name: `Symposium agent`,
            cwd: vscode.workspace.workspaceFolders?.[0].uri.fsPath
        });

        // Show the terminal
        terminal.show();

        // Send the agent command
        terminal.sendText(escapedCommand);

        outputChannel.appendLine('Agent launched successfully');

    } catch (error) {
        outputChannel.appendLine(`Error launching AI agent: ${error}`);
    }
}

export function activate(context: vscode.ExtensionContext) {

    // 💡: Create dedicated output channel for cleaner logging
    const outputChannel = vscode.window.createOutputChannel('Symposium');
    outputChannel.appendLine('Symposium extension is now active');
    console.log('Symposium extension is now active');

    // Create the central bus
    const bus = new Bus(context, outputChannel);

    // 💡: PID Discovery Testing - Log VSCode and terminal PIDs
    logPIDDiscovery(outputChannel).catch(error => {
        outputChannel.appendLine(`Error in PID discovery: ${error}`);
    });

    // Create synthetic PR provider for AI-generated pull requests
    const syntheticPRProvider = new SyntheticPRProvider(context);
    bus.setSyntheticPRProvider(syntheticPRProvider);

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

                // Show success message
                vscode.window.showInformationMessage('Comment reply inserted into AI chat');
            } catch (error) {
                console.error('Failed to reply to walkthrough comment:', error);
                vscode.window.showErrorMessage(`Failed to reply to comment: ${error}`);
            }
        }
    );
    context.subscriptions.push(walkthroughReplyCommand);

    console.log('Webview provider created successfully');

    // 💡: Set up daemon client connection for message bus communication
    const daemonClient = new DaemonClient(context, outputChannel, syntheticPRProvider, walkthroughProvider);
    bus.setDaemonClient(daemonClient);

    daemonClient.start();

    // 💡: Check for taskspace environment and auto-launch agent if needed
    // (Must be after DaemonClient is initialized)
    checkTaskspaceEnvironment(outputChannel, bus).catch(error => {
        outputChannel.appendLine(`Error in taskspace detection: ${error}`);
    });

    // Set up comment callback to send comments as feedback
    syntheticPRProvider.setCommentCallback((comment: string, filePath: string, lineNumber: number) => {
        daemonClient.handleCommentFeedback(comment, filePath, lineNumber);
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
        await logPIDDiscovery(outputChannel);
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

    context.subscriptions.push(showReviewCommand, reviewActionCommand, copyReviewCommand, logPIDsCommand, syntheticPRProvider, daemonClient, toggleWindowTitleCommand);

    // Return API for Ask Socratic Shell integration
    return {
        getActiveTerminals: () => daemonClient.getActiveTerminals()
    };
}

// 💡: Set up universal selection detection for interactive code review
function setupSelectionDetection(bus: Bus): void {
    const { context, outputChannel } = bus;

    outputChannel.appendLine('Setting up universal selection detection...');

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

    // 💡: Register Code Action Provider for "Socratic Shell" section
    const codeActionProvider = vscode.languages.registerCodeActionsProvider(
        '*', // All file types
        {
            provideCodeActions(document, range, context) {
                // Only show when there's a non-empty selection
                if (!range.isEmpty) {
                    const action = new vscode.CodeAction(
                        'Ask Socratic Shell',
                        vscode.CodeActionKind.QuickFix
                    );
                    action.command = {
                        command: 'symposium.chatAboutSelection',
                        title: 'Ask Socratic Shell'
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

            outputChannel.appendLine(`CHAT ICON CLICKED!`);
            outputChannel.appendLine(`Selected: "${selectedText}"`);
            outputChannel.appendLine(`Location: ${filePath}:${startLine}:${startColumn}-${endLine}:${endColumn}`);

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
                outputChannel.appendLine(`Compact reference sent for ${relativePath}:${startLine}`);
            } catch (error) {
                outputChannel.appendLine(`Failed to send reference: ${error}`);
                vscode.window.showErrorMessage('Failed to send reference to terminal');
            }
        } else {
            outputChannel.appendLine('Chat action triggered but no current selection found');
        }
    });

    context.subscriptions.push(selectionListener, codeActionProvider, chatIconCommand);
    outputChannel.appendLine('Selection detection with Code Actions setup complete');
}

// 💡: Phase 4 - Intelligent terminal detection using registry
async function findQChatTerminal(bus: Bus): Promise<vscode.Terminal | null> {
    const { outputChannel, context } = bus;
    const terminals = vscode.window.terminals;
    outputChannel.appendLine(`Found ${terminals.length} open terminals`);

    if (terminals.length === 0) {
        outputChannel.appendLine('No terminals found');
        return null;
    }

    // Get active terminals with MCP servers from registry
    const activeTerminals = bus.getActiveTerminals();
    outputChannel.appendLine(`Active MCP server terminals: [${Array.from(activeTerminals).join(', ')}]`);

    if (activeTerminals.size === 0) {
        outputChannel.appendLine('No terminals with active MCP servers found');
        return null;
    }

    // Filter terminals to only those with active MCP servers (async)
    const terminalChecks = await Promise.all(
        terminals.map(async (terminal) => {
            // Extract the shell PID from the terminal (async)
            const shellPID = await terminal.processId;

            // Log terminal for debugging
            outputChannel.appendLine(`  Checking terminal: "${terminal.name}" (PID: ${shellPID})`);

            // Check if this terminal's shell PID is in our active registry
            if (shellPID && activeTerminals.has(shellPID)) {
                outputChannel.appendLine(`    ✅ Terminal "${terminal.name}" has active MCP server (PID: ${shellPID})`);
                return { terminal, isAiEnabled: true };
            } else {
                outputChannel.appendLine(`    ❌ Terminal "${terminal.name}" has no active MCP server (PID: ${shellPID})`);
                return { terminal, isAiEnabled: false };
            }
        })
    );

    // Extract only the AI-enabled terminals
    const aiEnabledTerminals = terminalChecks
        .filter(check => check.isAiEnabled)
        .map(check => check.terminal);

    outputChannel.appendLine(`AI-enabled terminals found: ${aiEnabledTerminals.length}`);

    // 💡: Simple case - exactly one AI-enabled terminal
    if (aiEnabledTerminals.length === 1) {
        const terminal = aiEnabledTerminals[0];
        outputChannel.appendLine(`Using single AI-enabled terminal: ${terminal.name}`);
        return terminal;
    }

    // 💡: Multiple AI-enabled terminals - show picker UI with memory
    if (aiEnabledTerminals.length > 1) {
        outputChannel.appendLine(`Multiple AI-enabled terminals found: ${aiEnabledTerminals.length}`);

        // Get previously selected terminal PID from workspace state
        const lastSelectedPID = context.workspaceState.get<number>('symposium.lastSelectedTerminalPID');
        outputChannel.appendLine(`Last selected terminal PID: ${lastSelectedPID}`);

        // Create picker items with terminal info
        interface TerminalQuickPickItem extends vscode.QuickPickItem {
            terminal: vscode.Terminal;
            pid: number | undefined;
        }

        const terminalItems: TerminalQuickPickItem[] = await Promise.all(
            aiEnabledTerminals.map(async (terminal): Promise<TerminalQuickPickItem> => {
                const pid = await terminal.processId;
                const isLastSelected = pid === lastSelectedPID;
                return {
                    label: isLastSelected ? `$(star-full) ${terminal.name}` : terminal.name,
                    description: `PID: ${pid}${isLastSelected ? ' (last used)' : ''}`,
                    detail: 'Terminal with active MCP server',
                    terminal: terminal,
                    pid: pid
                };
            })
        );

        // Keep natural terminal order - don't sort, just use visual indicators

        // Find the last selected terminal for the quick option
        const lastSelectedItem = terminalItems.find(item => item.pid === lastSelectedPID);

        // Create picker items with optional "use last" entry at top
        const pickerItems: TerminalQuickPickItem[] = [];

        // Add "use last terminal" option if we have a previous selection
        if (lastSelectedItem) {
            pickerItems.push({
                label: `$(history) Use last terminal: ${lastSelectedItem.terminal.name}`,
                description: `PID: ${lastSelectedItem.pid}`,
                detail: 'Quick access to your previously used terminal',
                terminal: lastSelectedItem.terminal,
                pid: lastSelectedItem.pid
            });

            // Add separator
            pickerItems.push({
                label: '$(dash) All available terminals',
                description: '',
                detail: '',
                terminal: null as any, // This won't be selectable
                pid: undefined,
                kind: vscode.QuickPickItemKind.Separator
            });
        }

        // Add all terminals (keeping natural order)
        pickerItems.push(...terminalItems);

        // Show the picker to user
        const selectedItem = await vscode.window.showQuickPick(pickerItems, {
            placeHolder: lastSelectedItem
                ? 'Select terminal for AI chat (first option = quick access to last used)'
                : 'Select terminal for AI chat',
            title: 'Multiple AI-enabled terminals found'
        });

        if (selectedItem) {
            // Safety check - ignore separator selections
            if (selectedItem.kind === vscode.QuickPickItemKind.Separator || !selectedItem.terminal) {
                outputChannel.appendLine('User selected separator or invalid item, ignoring');
                return null;
            }

            outputChannel.appendLine(`User selected terminal: ${selectedItem.terminal.name} (PID: ${selectedItem.pid})`);

            // Remember this selection for next time
            await context.workspaceState.update('symposium.lastSelectedTerminalPID', selectedItem.pid);
            outputChannel.appendLine(`Saved terminal PID ${selectedItem.pid} as last selected`);

            return selectedItem.terminal;
        } else {
            outputChannel.appendLine('User cancelled terminal selection');
            return null;
        }
    }

    // 💡: No AI-enabled terminals found - fall back to old logic for compatibility
    outputChannel.appendLine('No AI-enabled terminals found, falling back to name-based detection');

    if (terminals.length === 1) {
        const terminal = terminals[0];
        outputChannel.appendLine(`Using single terminal (fallback): ${terminal.name}`);
        return terminal;
    }

    const targetTerminal = terminals.find(terminal => {
        const name = terminal.name.toLowerCase();
        return name.includes('socratic shell') || name.includes('ai');
    });

    if (targetTerminal) {
        outputChannel.appendLine(`Found target terminal (fallback): ${targetTerminal.name}`);
        return targetTerminal;
    }

    outputChannel.appendLine('Multiple terminals found, but none are AI-enabled or named appropriately');
    return null;
}

// 💡: Phase 5 - Create compact reference for selection context
async function createCompactSelectionReference(
    selectedText: string,
    filePath: string,
    startLine: number,
    startColumn: number,
    endLine: number,
    endColumn: number,
    bus: Bus
): Promise<string> {
    try {
        const relativePath = vscode.workspace.asRelativePath(filePath);
        const referenceId = crypto.randomUUID();

        // Create reference data matching the expected format
        const referenceData = {
            file: relativePath,
            line: startLine,
            selection: selectedText.length > 0 ? selectedText : undefined
        };

        // Store the reference using the bus
        await bus.sendReferenceToActiveShell(referenceId, referenceData);

        // Return compact reference tag
        const compactRef = `<symposium-ref id="${referenceId}"/>\n\n`;
        bus.log(`Created compact reference ${referenceId} for ${relativePath}:${startLine}`);

        return compactRef;
    } catch (error) {
        bus.log(`Failed to create compact reference: ${error}`);
        // Fallback to old format if compact reference fails
        return formatSelectionMessageLegacy(selectedText, filePath, startLine, startColumn, endLine, endColumn);
    }
}

// 💡: Legacy fallback - Format selection context for Q chat injection (old verbose format)
function formatSelectionMessageLegacy(
    selectedText: string,
    filePath: string,
    startLine: number,
    startColumn: number,
    endLine: number,
    endColumn: number
): string {
    // 💡: Create a formatted message that provides context to the AI
    const relativePath = vscode.workspace.asRelativePath(filePath);
    const location = startLine === endLine
        ? `${relativePath}:${startLine}:${startColumn}-${endColumn}`
        : `${relativePath}:${startLine}:${startColumn}-${endLine}:${endColumn}`;

    // 💡: Format as a natural message that user can continue typing after
    // 💡: Show just first 30 chars with escaped newlines for concise terminal display
    const escapedText = selectedText.replace(/\n/g, '\\n');
    const truncatedText = escapedText.length > 30
        ? escapedText.substring(0, 30) + '...'
        : escapedText;

    const message = `<context>looking at this code from ${location} <content>${truncatedText}</content></context> `;

    return message;
}

// 💡: PID Discovery Testing - Log all relevant PIDs for debugging
async function logPIDDiscovery(outputChannel: vscode.OutputChannel): Promise<void> {
    outputChannel.appendLine('=== PID DISCOVERY TESTING ===');

    // Extension process info
    outputChannel.appendLine(`Extension process PID: ${process.pid}`);
    outputChannel.appendLine(`Extension parent PID: ${process.ppid}`);

    // Try to find VSCode PID by walking up the process tree
    const vscodePid = findVSCodePID(outputChannel);
    if (vscodePid) {
        outputChannel.appendLine(`Found VSCode PID: ${vscodePid}`);
    } else {
        outputChannel.appendLine('Could not find VSCode PID');
    }

    // Log terminal PIDs (handle the Promise properly)
    const terminals = vscode.window.terminals;
    outputChannel.appendLine(`Found ${terminals.length} terminals:`);

    for (let i = 0; i < terminals.length; i++) {
        const terminal = terminals[i];
        try {
            // terminal.processId returns a Promise in newer VSCode versions
            const pid = await terminal.processId;
            outputChannel.appendLine(`  Terminal ${i}: name="${terminal.name}", PID=${pid}`);
        } catch (error) {
            outputChannel.appendLine(`  Terminal ${i}: name="${terminal.name}", PID=<error: ${error}>`);
        }
    }

    // Set up terminal monitoring
    const terminalListener = vscode.window.onDidOpenTerminal(async (terminal) => {
        try {
            const pid = await terminal.processId;
            outputChannel.appendLine(`NEW TERMINAL: name="${terminal.name}", PID=${pid}`);
        } catch (error) {
            outputChannel.appendLine(`NEW TERMINAL: name="${terminal.name}", PID=<error: ${error}>`);
        }
    });

    outputChannel.appendLine('=== END PID DISCOVERY ===');
}

// 💡: Attempt to find VSCode PID by walking up process tree
function findVSCodePID(outputChannel: vscode.OutputChannel): number | null {
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
                    outputChannel.appendLine(`Found VSCode PID: ${pid}`);
                    return pid;
                }

                currentPid = ppid;
                if (ppid <= 1) break; // Reached init process

            } catch (error) {
                break;
            }
        }

        outputChannel.appendLine('Could not find VSCode PID in process tree');
        return null;

    } catch (error) {
        outputChannel.appendLine(`PID discovery error: ${error}`);
        return null;
    }
}

export function deactivate() { }