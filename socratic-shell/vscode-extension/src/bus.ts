import * as vscode from 'vscode';
import * as crypto from 'crypto';
import { DaemonClient } from './ipc';

import { WalkthroughWebviewProvider } from './walkthroughWebview';
import { StructuredLogger } from './structuredLogger';
import { getCurrentTaskspaceUuid } from './taskspaceUtils';

/**
 * Central message bus for extension components
 * Reduces tight coupling by providing shared access to all major components
 */
export class Bus {
    public context: vscode.ExtensionContext;
    public outputChannel: vscode.OutputChannel;
    private logger: StructuredLogger;
    private _daemonClient: DaemonClient | undefined;
    private _walkthroughProvider: WalkthroughWebviewProvider | undefined;

    constructor(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
        this.context = context;
        this.outputChannel = outputChannel;
        this.logger = new StructuredLogger(outputChannel, 'EXTENSION-BUS');
    }

    // Register components as they're created
    setDaemonClient(client: DaemonClient) {
        this._daemonClient = client;
    }

    setWalkthroughProvider(provider: WalkthroughWebviewProvider) {
        this._walkthroughProvider = provider;
    }

    // Accessors with assertions
    get daemonClient(): DaemonClient {
        if (!this._daemonClient) {
            throw new Error('DaemonClient not initialized on Bus');
        }
        return this._daemonClient;
    }

    get walkthroughProvider(): WalkthroughWebviewProvider {
        if (!this._walkthroughProvider) {
            throw new Error('WalkthroughWebviewProvider not initialized on Bus');
        }
        return this._walkthroughProvider;
    }

    /**
     * Select an active AI-enabled terminal with picker for ambiguity resolution
     * Returns null if no suitable terminal found or user cancelled
     */
    private async selectActiveTerminal(): Promise<{ terminal: vscode.Terminal; shellPID: number } | null> {
        const terminals = vscode.window.terminals;
        if (terminals.length === 0) {
            vscode.window.showWarningMessage('No terminals found. Please open a terminal with an active AI assistant.');
            return null;
        }

        // Discover active MCP servers using MARCO/POLO
        const discoveredShells = await this.daemonClient.discoverActiveShells();
        this.log(`Discovered MCP server terminals: [${Array.from(discoveredShells.keys()).join(', ')}]`);

        if (discoveredShells.size === 0) {
            vscode.window.showWarningMessage('No terminals with active MCP servers found. Please ensure you have a terminal with an active AI assistant (like Q chat or Claude CLI) running.');
            return null;
        }

        // Get current taskspace for filtering
        const currentTaskspaceUuid = getCurrentTaskspaceUuid();

        // Filter shells by taskspace if we're in one
        let candidateShells = Array.from(discoveredShells.entries());
        if (currentTaskspaceUuid) {
            const taskspaceMatches = candidateShells.filter(([pid, payload]) =>
                payload.taskspace_uuid === currentTaskspaceUuid
            );
            if (taskspaceMatches.length > 0) {
                candidateShells = taskspaceMatches;
                this.log(`Filtered to ${candidateShells.length} shells matching taskspace ${currentTaskspaceUuid}`);
            }
        }

        // Filter terminals to only those with active MCP servers
        const terminalChecks = await Promise.all(
            terminals.map(async (terminal) => {
                const shellPID = await terminal.processId;
                const hasActiveShell = shellPID && candidateShells.some(([pid]) => pid === shellPID);
                return { terminal, shellPID, hasActiveShell };
            })
        );

        const aiEnabledTerminals = terminalChecks
            .filter(check => check.hasActiveShell && check.shellPID)
            .map(check => ({ terminal: check.terminal, shellPID: check.shellPID! }));

        if (aiEnabledTerminals.length === 0) {
            vscode.window.showWarningMessage('No AI-enabled terminals found in current context. Please ensure you have a terminal with an active MCP server running.');
            return null;
        }

        // Simple case - exactly one AI-enabled terminal
        if (aiEnabledTerminals.length === 1) {
            return aiEnabledTerminals[0];
        }

        // Multiple terminals - prefer currently active terminal if possible
        const activeTerminal = vscode.window.activeTerminal;
        if (activeTerminal) {
            const activeTerminalPID = await activeTerminal.processId;
            const activeMatch = aiEnabledTerminals.find(({ shellPID }) => shellPID === activeTerminalPID);
            if (activeMatch) {
                this.log(`Using currently active terminal ${activeMatch.shellPID}`);
                return activeMatch;
            }
        }

        // Still multiple terminals - show picker for ambiguity resolution
        const terminalItems = aiEnabledTerminals.map(({ terminal, shellPID }) => ({
            label: terminal.name,
            description: `PID: ${shellPID}`,
            terminal,
            shellPID
        }));

        const selected = await vscode.window.showQuickPick(terminalItems, {
            placeHolder: 'Select terminal'
        });

        return selected ? { terminal: selected.terminal, shellPID: selected.shellPID } : null;
    }

    /**
     * Send reference data to active terminal with consolidated logic
     * Handles terminal finding, reference creation, and XML generation
     */
    async sendToActiveTerminal(referenceData: any, options: { includeNewline: boolean }): Promise<void> {
        const selectedTerminal = await this.selectActiveTerminal();
        if (!selectedTerminal) return;

        // Generate fresh UUID for reference
        const referenceId = crypto.randomUUID();

        // Send reference data to MCP server for selected terminal
        this.daemonClient.sendStoreReferenceToShell(selectedTerminal.shellPID, referenceId, referenceData);
        this.log(`Reference ${referenceId} sent to shell ${selectedTerminal.shellPID}`);

        // Generate <symposium-ref id="..."/> XML (using current format)
        const xmlMessage = `<symposium-ref id="${referenceId}"/>` + (options.includeNewline ? '\n\n' : ' ');

        // Send XML to terminal
        selectedTerminal.terminal.sendText(xmlMessage, false); // false = don't execute, just insert text
        selectedTerminal.terminal.show(); // Bring terminal into focus

        this.log(`Reference ${referenceId} sent to terminal ${selectedTerminal.terminal.name} (PID: ${selectedTerminal.shellPID})`);
    }

    /**
     * Send plain text message to active terminal (no reference creation)
     * For simple text messages that don't need MCP reference storage
     */
    async sendTextToActiveTerminal(message: string): Promise<void> {
        const selectedTerminal = await this.selectActiveTerminal();
        if (!selectedTerminal) return;

        // Send text directly to terminal
        selectedTerminal.terminal.sendText(message, false); // false = don't execute, just insert text
        selectedTerminal.terminal.show(); // Bring terminal into focus

        this.log(`Text message sent to terminal ${selectedTerminal.terminal.name} (PID: ${selectedTerminal.shellPID})`);
    }

    log(message: string) {
        // Check if message is already structured (has [COMPONENT:PID] prefix)
        if (message.match(/^\[[A-Z-]+:\d+\]/)) {
            // Already structured, use as-is
            this.outputChannel.appendLine(message);
        } else {
            // Not structured, add our prefix
            this.logger.info(message);
        }
    }
}
