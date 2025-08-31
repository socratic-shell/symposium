// 💡: URL parser for dialectic: scheme supporting flexible search and line parameters
// Handles dialectic:path?search=term&line=N|N:C|N-M|N:C-M:D format as designed in issue #2

export interface DialecticUrl {
    path: string;
    regex?: string;
    line?: LineSpec;
}

export interface LineSpec {
    type: 'single' | 'range' | 'single-with-column' | 'range-with-columns';
    startLine: number;
    startColumn?: number;
    endLine?: number;
    endColumn?: number;
}

/**
 * Parse a dialectic: URL into its components
 * 
 * Supported formats:
 * - dialectic:path/to/file.ts
 * - dialectic:path/to/file.ts?regex=pattern
 * - dialectic:path/to/file.ts?line=42
 * - dialectic:path/to/file.ts?regex=pattern&line=42
 * - dialectic:path/to/file.ts?line=42:10 (line with column)
 * - dialectic:path/to/file.ts?line=42-50 (line range)
 * - dialectic:path/to/file.ts?line=42:10-50:20 (precise range)
 */
export function parseDialecticUrl(url: string): DialecticUrl | null {
    // 💡: Remove dialectic: prefix and validate scheme
    if (!url.startsWith('dialectic:')) {
        return null;
    }
    
    const urlWithoutScheme = url.substring('dialectic:'.length);
    
    // 💡: Split path from query parameters
    const [path, queryString] = urlWithoutScheme.split('?', 2);
    
    if (!path) {
        return null;
    }
    
    const result: DialecticUrl = { path };
    
    // 💡: Parse query parameters if present
    if (queryString) {
        const params = new URLSearchParams(queryString);
        
        // Handle regex parameter
        const regex = params.get('regex');
        if (regex) {
            result.regex = regex;
        }
        
        // Handle line parameter
        const line = params.get('line');
        if (line) {
            const lineSpec = parseLineSpec(line);
            if (lineSpec) {
                result.line = lineSpec;
            }
        }
    }
    
    return result;
}

/**
 * Parse line specification into structured format
 * 
 * Supported formats:
 * - "42" -> single line
 * - "42:10" -> line with column
 * - "42-50" -> line range
 * - "42:10-50:20" -> precise character range
 */
function parseLineSpec(lineStr: string): LineSpec | null {
    // 💡: Handle range with columns: 42:10-50:20
    const rangeWithColumnsMatch = lineStr.match(/^(\d+):(\d+)-(\d+):(\d+)$/);
    if (rangeWithColumnsMatch) {
        return {
            type: 'range-with-columns',
            startLine: parseInt(rangeWithColumnsMatch[1]),
            startColumn: parseInt(rangeWithColumnsMatch[2]),
            endLine: parseInt(rangeWithColumnsMatch[3]),
            endColumn: parseInt(rangeWithColumnsMatch[4])
        };
    }
    
    // 💡: Handle line range: 42-50
    const rangeMatch = lineStr.match(/^(\d+)-(\d+)$/);
    if (rangeMatch) {
        return {
            type: 'range',
            startLine: parseInt(rangeMatch[1]),
            endLine: parseInt(rangeMatch[2])
        };
    }
    
    // 💡: Handle single line with column: 42:10
    const singleWithColumnMatch = lineStr.match(/^(\d+):(\d+)$/);
    if (singleWithColumnMatch) {
        return {
            type: 'single-with-column',
            startLine: parseInt(singleWithColumnMatch[1]),
            startColumn: parseInt(singleWithColumnMatch[2])
        };
    }
    
    // 💡: Handle single line: 42
    const singleMatch = lineStr.match(/^(\d+)$/);
    if (singleMatch) {
        return {
            type: 'single',
            startLine: parseInt(singleMatch[1])
        };
    }
    
    return null;
}

/**
 * Convert a DialecticUrl back to string format
 * Useful for debugging and testing
 */
export function formatDialecticUrl(dialecticUrl: DialecticUrl): string {
    let url = `dialectic:${dialecticUrl.path}`;
    
    const params = new URLSearchParams();
    
    if (dialecticUrl.regex) {
        params.set('regex', dialecticUrl.regex);
    }
    
    if (dialecticUrl.line) {
        params.set('line', formatLineSpec(dialecticUrl.line));
    }
    
    const queryString = params.toString();
    if (queryString) {
        url += `?${queryString}`;
    }
    
    return url;
}

/**
 * Convert LineSpec back to string format
 */
function formatLineSpec(lineSpec: LineSpec): string {
    switch (lineSpec.type) {
        case 'single':
            return lineSpec.startLine.toString();
        case 'single-with-column':
            return `${lineSpec.startLine}:${lineSpec.startColumn}`;
        case 'range':
            return `${lineSpec.startLine}-${lineSpec.endLine}`;
        case 'range-with-columns':
            return `${lineSpec.startLine}:${lineSpec.startColumn}-${lineSpec.endLine}:${lineSpec.endColumn}`;
    }
}
