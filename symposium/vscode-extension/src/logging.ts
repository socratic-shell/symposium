/**
 * Convenience logging functions for the extension
 * These functions use the global logger when available, fallback to console
 */

import { getLogger } from './extension';
import { LogOptions } from './structuredLogger';

export function debugLog(message: string, options?: LogOptions): void {
    const logger = getLogger();
    if (logger) {
        logger.debug(message, options);
    } else {
        console.log(`[DEBUG] ${message}`);
    }
}

