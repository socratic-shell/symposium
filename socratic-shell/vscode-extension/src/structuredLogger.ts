/**
 * Structured logging utility for VSCode extension components
 * Provides consistent log formatting with component and process ID prefixes
 * Sends logs both to VSCode output channel and daemon subscribers
 */

import * as vscode from 'vscode';

export enum LogLevel {
    DEBUG = 'debug',
    INFO = 'info', 
    WARN = 'warn',
    ERROR = 'error'
}

// Interface for daemon log messages
interface LogMessage {
    level: string;
    message: string;
}

// Interface for daemon client (minimal interface for logging)
interface IDaemonClient {
    sendRequestNoReply(type: string, payload: any): Promise<string>
}

// Options that can be given to the logger
export interface LogOptions {
    // Do not log in the daemon; used for logging related to sending IPC messages
    local?: boolean
}

export class StructuredLogger {
    private readonly component: string;
    private readonly pid: number;
    private daemonClient: IDaemonClient | null = null;

    constructor(
        private readonly outputChannel: vscode.OutputChannel,
        component: string = 'EXTENSION'
    ) {
        this.component = component;
        this.pid = process.pid;
    }

    /**
     * Set the daemon client for sending logs to daemon subscribers
     */
    setDaemonClient(client: IDaemonClient): void {
        this.daemonClient = client;
    }

    private async sendToDaemon(level: LogLevel, message: string): Promise<void> {
        if (this.daemonClient) {
            try {
                const logMessage: LogMessage = { level, message };
                await this.daemonClient.sendRequestNoReply('log', logMessage);
            } catch (error) {
                // Silently fail daemon logging to avoid infinite loops
                // The output channel will still receive the message
            }
        }
    }

    debug(message: string, options?: LogOptions): void {
        this.log(LogLevel.DEBUG, message, options);
    }

    info(message: string, options?: LogOptions): void {
        this.log(LogLevel.INFO, message, options);
    }

    warn(message: string, options?: LogOptions): void {
        this.log(LogLevel.WARN, message, options);
    }

    error(message: string, options?: LogOptions): void {
        this.log(LogLevel.ERROR, message, options);
    }

    /**
     * Log with explicit level (useful for dynamic logging)
     */
    log(level: LogLevel, message: string, options?: LogOptions): void {
        this.outputChannel.appendLine(`[${level}] ${message}`);
        if (!options?.local) {
            this.sendToDaemon(level, message);            
        }
    }

    /**
     * Create a prefixed logger for a sub-component
     * Example: logger.sub('IPC') creates logger with component 'EXTENSION-IPC'
     */
    sub(subComponent: string): StructuredLogger {
        const subLogger = new StructuredLogger(this.outputChannel, `${this.component}-${subComponent}`);
        if (this.daemonClient) {
            subLogger.setDaemonClient(this.daemonClient);
        }
        return subLogger;
    }
}
