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
    sendRequest<T>(type: string, payload: any, timeoutMs?: number): Promise<T | null>;
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

    private formatMessage(level: LogLevel, message: string): string {
        return `[${this.component}:${this.pid}] ${level.toUpperCase()} ${message}`;
    }

    private async sendToDaemon(level: LogLevel, message: string): Promise<void> {
        if (this.daemonClient) {
            try {
                const logMessage: LogMessage = {
                    level: level,
                    message: `[${this.component}:${this.pid}] ${message}`
                };
                await this.daemonClient.sendRequest('log', logMessage);
            } catch (error) {
                // Silently fail daemon logging to avoid infinite loops
                // The output channel will still receive the message
            }
        }
    }

    debug(message: string): void {
        const formatted = this.formatMessage(LogLevel.DEBUG, message);
        this.outputChannel.appendLine(formatted);
        this.sendToDaemon(LogLevel.DEBUG, message);
    }

    info(message: string): void {
        const formatted = this.formatMessage(LogLevel.INFO, message);
        this.outputChannel.appendLine(formatted);
        this.sendToDaemon(LogLevel.INFO, message);
    }

    warn(message: string): void {
        const formatted = this.formatMessage(LogLevel.WARN, message);
        this.outputChannel.appendLine(formatted);
        this.sendToDaemon(LogLevel.WARN, message);
    }

    error(message: string): void {
        const formatted = this.formatMessage(LogLevel.ERROR, message);
        this.outputChannel.appendLine(formatted);
        this.sendToDaemon(LogLevel.ERROR, message);
    }

    /**
     * Log with explicit level (useful for dynamic logging)
     */
    log(level: LogLevel, message: string): void {
        const formatted = this.formatMessage(level, message);
        this.outputChannel.appendLine(formatted);
        this.sendToDaemon(level, message);
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

/**
 * Factory function to create a structured logger
 */
export function createStructuredLogger(
    outputChannel: vscode.OutputChannel,
    component: string = 'EXTENSION'
): StructuredLogger {
    return new StructuredLogger(outputChannel, component);
}