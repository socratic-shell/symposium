/**
 * Structured logging utility for VSCode extension components
 * Provides consistent log formatting with component and process ID prefixes
 */

import * as vscode from 'vscode';

export enum LogLevel {
    DEBUG = 'DEBUG',
    INFO = 'INFO',
    WARN = 'WARN',
    ERROR = 'ERROR'
}

export class StructuredLogger {
    private readonly component: string;
    private readonly pid: number;

    constructor(
        private readonly outputChannel: vscode.OutputChannel,
        component: string = 'EXTENSION'
    ) {
        this.component = component;
        this.pid = process.pid;
    }

    private formatMessage(level: LogLevel, message: string): string {
        return `[${this.component}:${this.pid}] ${level} ${message}`;
    }

    debug(message: string): void {
        const formatted = this.formatMessage(LogLevel.DEBUG, message);
        this.outputChannel.appendLine(formatted);
    }

    info(message: string): void {
        const formatted = this.formatMessage(LogLevel.INFO, message);
        this.outputChannel.appendLine(formatted);
    }

    warn(message: string): void {
        const formatted = this.formatMessage(LogLevel.WARN, message);
        this.outputChannel.appendLine(formatted);
    }

    error(message: string): void {
        const formatted = this.formatMessage(LogLevel.ERROR, message);
        this.outputChannel.appendLine(formatted);
    }

    /**
     * Log with explicit level (useful for dynamic logging)
     */
    log(level: LogLevel, message: string): void {
        const formatted = this.formatMessage(level, message);
        this.outputChannel.appendLine(formatted);
    }

    /**
     * Create a prefixed logger for a sub-component
     * Example: logger.sub('IPC') creates logger with component 'EXTENSION-IPC'
     */
    sub(subComponent: string): StructuredLogger {
        return new StructuredLogger(this.outputChannel, `${this.component}-${subComponent}`);
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