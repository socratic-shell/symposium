"use strict";
// 💡: Search engine for finding text within files with optional line constraints
// Supports the parameter combinations from dialectic: URL scheme design
Object.defineProperty(exports, "__esModule", { value: true });
exports.formatSearchResults = exports.needsDisambiguation = exports.getBestSearchResult = exports.searchInFile = void 0;
const vscode = require("vscode");
/**
 * Search for text within a file, optionally constrained to specific lines
 * Supports multi-line regex patterns using VSCode's position mapping APIs
 *
 * Parameter combinations:
 * - regex=pattern -> search entire file
 * - regex=pattern&line=100 -> search starting from line 100
 * - regex=pattern&line=50-150 -> search only within lines 50-150
 */
async function searchInFile(fileUri, options) {
    try {
        // 💡: Use VSCode's TextDocument API for position mapping and content access
        const document = await vscode.workspace.openTextDocument(fileUri);
        const { regexPattern, lineConstraint, caseSensitive = false } = options;
        // 💡: Get search content based on line constraints
        const searchContent = getSearchContent(document, lineConstraint);
        const searchStartOffset = searchContent.startOffset;
        // 💡: Create regex with multiline support
        const flags = caseSensitive ? 'gm' : 'gim'; // Added 'm' flag for multiline
        let regex;
        try {
            regex = new RegExp(regexPattern, flags);
        }
        catch (error) {
            throw new Error(`Invalid regex pattern "${regexPattern}": ${error}`);
        }
        console.log(`[SearchEngine] Searching with regex: /${regexPattern}/${flags}`);
        console.log(`[SearchEngine] Search content length: ${searchContent.text.length} chars, offset: ${searchStartOffset}`);
        const results = [];
        // 💡: Search the full content (supports multi-line patterns)
        let match;
        while ((match = regex.exec(searchContent.text)) !== null) {
            const matchStart = match.index;
            const matchLength = match[0].length;
            const absoluteOffset = searchStartOffset + matchStart;
            // 💡: Use VSCode's positionAt to convert offset to line/column
            const startPosition = document.positionAt(absoluteOffset);
            console.log(`[SearchEngine] Found match at offset ${absoluteOffset}: "${match[0].substring(0, 50)}${match[0].length > 50 ? '...' : ''}"`);
            console.log(`[SearchEngine] Position: line ${startPosition.line + 1}, column ${startPosition.character + 1}`);
            // 💡: Check if match falls within line constraints
            if (isMatchWithinConstraints(startPosition, lineConstraint)) {
                // 💡: Extract the line containing the match start for display
                const matchLine = document.lineAt(startPosition.line);
                results.push({
                    line: startPosition.line + 1,
                    column: startPosition.character + 1,
                    text: matchLine.text,
                    matchStart: startPosition.character,
                    matchEnd: match[0].includes('\n') ? matchLine.text.length : startPosition.character + matchLength
                });
            }
            else {
                console.log(`[SearchEngine] Match excluded by line constraints`);
            }
            // 💡: Prevent infinite loop on zero-width matches
            if (matchLength === 0) {
                regex.lastIndex++;
            }
        }
        console.log(`[SearchEngine] Total matches found: ${results.length}`);
        return results;
    }
    catch (error) {
        throw new Error(`Failed to search in file ${fileUri.fsPath}: ${error}`);
    }
}
exports.searchInFile = searchInFile;
/**
 * Get search content based on line constraints, using VSCode's position mapping
 */
function getSearchContent(document, lineConstraint) {
    if (!lineConstraint) {
        // 💡: Search entire document
        return { text: document.getText(), startOffset: 0 };
    }
    // 💡: Convert line constraints to VSCode Range and get text within that range
    const startLine = Math.max(0, lineConstraint.startLine - 1); // Convert to 0-based
    const startChar = lineConstraint.startColumn ? lineConstraint.startColumn - 1 : 0; // Convert to 0-based
    let endLine;
    let endChar;
    switch (lineConstraint.type) {
        case 'single':
            // 💡: For single line, search from that line to end of document
            endLine = document.lineCount - 1;
            endChar = document.lineAt(endLine).text.length;
            break;
        case 'single-with-column':
            // 💡: For single line with column, search from that position to end
            endLine = document.lineCount - 1;
            endChar = document.lineAt(endLine).text.length;
            break;
        case 'range':
            // 💡: For range, search only within the specified lines
            endLine = Math.min(document.lineCount - 1, (lineConstraint.endLine || lineConstraint.startLine) - 1);
            endChar = document.lineAt(endLine).text.length;
            break;
        case 'range-with-columns':
            // 💡: For precise range, use exact boundaries
            endLine = Math.min(document.lineCount - 1, (lineConstraint.endLine || lineConstraint.startLine) - 1);
            endChar = lineConstraint.endColumn ? lineConstraint.endColumn - 1 : document.lineAt(endLine).text.length;
            break;
    }
    const startPosition = new vscode.Position(startLine, startChar);
    const endPosition = new vscode.Position(endLine, endChar);
    const range = new vscode.Range(startPosition, endPosition);
    return {
        text: document.getText(range),
        startOffset: document.offsetAt(startPosition)
    };
}
/**
 * Check if a match position falls within line constraints
 */
function isMatchWithinConstraints(position, lineConstraint) {
    if (!lineConstraint) {
        return true;
    }
    const line = position.line + 1; // Convert to 1-based
    const column = position.character + 1; // Convert to 1-based
    // 💡: Check line bounds
    if (line < lineConstraint.startLine) {
        return false;
    }
    if (lineConstraint.endLine && line > lineConstraint.endLine) {
        return false;
    }
    // 💡: Check column bounds for single line with column constraint
    if (lineConstraint.type === 'single-with-column' && line === lineConstraint.startLine) {
        return column >= (lineConstraint.startColumn || 1);
    }
    // 💡: Check column bounds for range with columns
    if (lineConstraint.type === 'range-with-columns') {
        if (line === lineConstraint.startLine && column < (lineConstraint.startColumn || 1)) {
            return false;
        }
        if (line === lineConstraint.endLine && column > (lineConstraint.endColumn || Number.MAX_SAFE_INTEGER)) {
            return false;
        }
    }
    return true;
}
/**
 * Get the best search result for navigation
 * Returns the first match for single results, null for empty results
 */
function getBestSearchResult(results) {
    return results.length > 0 ? results[0] : null;
}
exports.getBestSearchResult = getBestSearchResult;
/**
 * Check if search results need disambiguation (multiple matches)
 */
function needsDisambiguation(results) {
    return results.length > 1;
}
exports.needsDisambiguation = needsDisambiguation;
/**
 * Format search results for debugging/logging
 */
function formatSearchResults(results) {
    if (results.length === 0) {
        return 'No matches found';
    }
    return results.map(result => `${result.line}:${result.column} "${result.text.trim()}"`).join('\n');
}
exports.formatSearchResults = formatSearchResults;
//# sourceMappingURL=searchEngine.js.map