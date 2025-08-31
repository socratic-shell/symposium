"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Bus = void 0;
const vscode = require("vscode");
const crypto = require("crypto");
/**
 * Central message bus for extension components
 * Reduces tight coupling by providing shared access to all major components
 */
class Bus {
    constructor(context, outputChannel) {
        this.context = context;
        this.outputChannel = outputChannel;
    }
    // Register components as they're created
    setDaemonClient(client) {
        this._daemonClient = client;
    }
    setSyntheticPRProvider(provider) {
        this._syntheticPRProvider = provider;
    }
    setWalkthroughProvider(provider) {
        this._walkthroughProvider = provider;
    }
    // Accessors with assertions
    get daemonClient() {
        if (!this._daemonClient) {
            throw new Error('DaemonClient not initialized on Bus');
        }
        return this._daemonClient;
    }
    get syntheticPRProvider() {
        if (!this._syntheticPRProvider) {
            throw new Error('SyntheticPRProvider not initialized on Bus');
        }
        return this._syntheticPRProvider;
    }
    get walkthroughProvider() {
        if (!this._walkthroughProvider) {
            throw new Error('WalkthroughWebviewProvider not initialized on Bus');
        }
        return this._walkthroughProvider;
    }
    // Convenience methods for common operations
    async sendReferenceToActiveShell(referenceId, referenceData) {
        return this.daemonClient.sendReferenceToActiveShell(referenceId, referenceData);
    }
    /**
     * Select an active AI-enabled terminal with picker for ambiguity resolution
     * Returns null if no suitable terminal found or user cancelled
     */
    async selectActiveTerminal() {
        const terminals = vscode.window.terminals;
        if (terminals.length === 0) {
            vscode.window.showWarningMessage('No terminals found. Please open a terminal with an active AI assistant.');
            return null;
        }
        const activeTerminals = this.getActiveTerminals();
        this.log(`Active MCP server terminals: [${Array.from(activeTerminals).join(', ')}]`);
        if (activeTerminals.size === 0) {
            vscode.window.showWarningMessage('No terminals with active MCP servers found. Please ensure you have a terminal with an active AI assistant (like Q chat or Claude CLI) running.');
            return null;
        }
        // Filter terminals to only those with active MCP servers
        const terminalChecks = await Promise.all(terminals.map(async (terminal) => {
            const shellPID = await terminal.processId;
            const isAiEnabled = shellPID && activeTerminals.has(shellPID);
            return { terminal, shellPID, isAiEnabled };
        }));
        const aiEnabledTerminals = terminalChecks
            .filter(check => check.isAiEnabled && check.shellPID)
            .map(check => ({ terminal: check.terminal, shellPID: check.shellPID }));
        if (aiEnabledTerminals.length === 0) {
            vscode.window.showWarningMessage('No AI-enabled terminals found. Please ensure you have a terminal with an active MCP server running.');
            return null;
        }
        // Simple case - exactly one AI-enabled terminal
        if (aiEnabledTerminals.length === 1) {
            return aiEnabledTerminals[0];
        }
        // Multiple terminals - show picker for ambiguity resolution
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
    async sendToActiveTerminal(referenceData, options) {
        const selectedTerminal = await this.selectActiveTerminal();
        if (!selectedTerminal)
            return;
        // Generate fresh UUID for reference
        const referenceId = crypto.randomUUID();
        // Send reference data to MCP server for selected terminal
        if (selectedTerminal.shellPID) {
            await this.sendReferenceToActiveShell(referenceId, referenceData);
        }
        // Generate <ssref id="..."/> XML (using current format)
        const xmlMessage = `<ssref id="${referenceId}"/>` + (options.includeNewline ? '\n\n' : '');
        // Send XML to terminal
        selectedTerminal.terminal.sendText(xmlMessage, false); // false = don't execute, just insert text
        selectedTerminal.terminal.show(); // Bring terminal into focus
        this.log(`Reference ${referenceId} sent to terminal ${selectedTerminal.terminal.name} (PID: ${selectedTerminal.shellPID})`);
    }
    /**
     * Send plain text message to active terminal (no reference creation)
     * For simple text messages that don't need MCP reference storage
     */
    async sendTextToActiveTerminal(message) {
        const selectedTerminal = await this.selectActiveTerminal();
        if (!selectedTerminal)
            return;
        // Send text directly to terminal
        selectedTerminal.terminal.sendText(message, false); // false = don't execute, just insert text
        selectedTerminal.terminal.show(); // Bring terminal into focus
        this.log(`Text message sent to terminal ${selectedTerminal.terminal.name} (PID: ${selectedTerminal.shellPID})`);
    }
    getActiveTerminals() {
        return this.daemonClient.getActiveTerminals();
    }
    log(message) {
        this.outputChannel.appendLine(message);
    }
}
exports.Bus = Bus;
//# sourceMappingURL=bus.js.map