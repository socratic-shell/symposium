// 💡: URL parser for symposium: scheme supporting flexible search and line parameters
// Handles symposium:path?search=term&line=N|N:C|N-M|N:C-M:D format as designed in issue #2

export interface SymposiumUrl {
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
 * Parse a symposium: URL into its components
 * 
 * Supported formats:
 * - symposium:path/to/file.ts
 * - symposium:path/to/file.ts?regex=pattern
 * - symposium:path/to/file.ts?line=42
 * - symposium:path/to/file.ts?regex=pattern&line=42
 * - symposium:path/to/file.ts?line=42:10 (line with column)
 * - symposium:path/to/file.ts?line=42-50 (line range)
 * - symposium:path/to/file.ts?line=42:10-50:20 (precise range)
 */
export function parseSymposiumUrl(url: string): SymposiumUrl | null {
    // 💡: Remove symposium: prefix and validate scheme
    if (!url.startsWith('symposium:')) {
        return null;
    }
    
    const urlWithoutScheme = url.substring('symposium:'.length);
    
    // 💡: Split path from query parameters
    const [path, queryString] = urlWithoutScheme.split('?', 2);
    
    if (!path) {
        return null;
    }
    
    const result: SymposiumUrl = { path };
    
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
 * Convert a SocratiShellUrl back to string format
 * Useful for debugging and testing
 */
export function formatSymposiumUrl(symposiumUrl: SymposiumUrl): string {
    let url = `symposium:${symposiumUrl.path}`;
    
    const params = new URLSearchParams();
    
    if (symposiumUrl.regex) {
        params.set('regex', symposiumUrl.regex);
    }
    
    if (symposiumUrl.line) {
        params.set('line', formatLineSpec(symposiumUrl.line));
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
