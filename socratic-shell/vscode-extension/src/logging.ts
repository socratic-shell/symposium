/**
 * Convenience logging functions for the extension
 * These functions use the global logger when available, fallback to console
 */

import { getLogger } from './extension';

export function debugLog(message: string): void {
    const logger = getLogger();
    if (logger) {
        logger.debug(message);
    } else {
        console.log(`[DEBUG] ${message}`);
    }
}

export function infoLog(message: string): void {
    const logger = getLogger();
    if (logger) {
        logger.info(message);
    } else {
        console.log(`[INFO] ${message}`);
    }
}

export function warnLog(message: string): void {
    const logger = getLogger();
    if (logger) {
        logger.warn(message);
    } else {
        console.warn(`[WARN] ${message}`);
    }
}

export function errorLog(message: string): void {
    const logger = getLogger();
    if (logger) {
        logger.error(message);
    } else {
        console.error(`[ERROR] ${message}`);
    }
}
