import * as vscode from 'vscode';
import * as path from 'path';
import { debugLog } from './logging';
import { parseSymposiumUrl, SymposiumUrl } from './symposiumUrl';
import { searchInFile, getBestSearchResult, formatSearchResults, needsDisambiguation } from './searchEngine';

// Placement state for unified link and comment management
interface PlacementState {
    isPlaced: boolean;
    chosenLocation: any; // FileRange, SearchResult, or other location type
    wasAmbiguous: boolean; // Whether this item had multiple possible locations
}

/**
 * Resolve a symposium URL to a precise location, using placement memory and user disambiguation
 * Returns the resolved location without navigating to it
 */
export async function resolveSymposiumUrlPlacement(
    symposiumUrl: string,
    baseUri?: vscode.Uri,
    placementMemory?: Map<string, PlacementState>
): Promise<{ range: vscode.Range; document: vscode.TextDocument } | null> {
    try {
        // Parse the symposium URL to extract components
        const parsed = parseSymposiumUrl(symposiumUrl);
        if (!parsed) {
            vscode.window.showErrorMessage(`Invalid symposium URL: ${symposiumUrl}`);
            return null;
        }

        debugLog(`Resolving symposium URL: ${symposiumUrl}`);
        debugLog(`Parsed components: ${JSON.stringify(parsed, null, 2)}`);

        // Resolve the file path
        let resolvedPath = parsed.path;
        if (baseUri && !path.isAbsolute(parsed.path)) {
            resolvedPath = path.resolve(baseUri.fsPath, parsed.path);
        }

        debugLog(`Resolved file path: ${resolvedPath}`);

        // Open the document
        const fileUri = vscode.Uri.file(resolvedPath);
        const document = await vscode.workspace.openTextDocument(fileUri);
        let targetLine = 1;
        let targetColumn = 1;

        // Handle regex search if present
        if (parsed.regex) {
            try {
                const searchResults = await searchInFile(fileUri, { regexPattern: parsed.regex });
                debugLog(`Search results: ${formatSearchResults(searchResults)}`);

                if (searchResults.length === 0) {
                    vscode.window.showWarningMessage(`Regex pattern "${parsed.regex}" not found in ${parsed.path}`);
                    // Fall back to line parameter if regex fails
                    if (parsed.line) {
                        targetLine = parsed.line.startLine;
                        targetColumn = parsed.line.startColumn || 1;
                    }
                } else {
                    // Check if we have a stored placement
                    const linkKey = `link:${symposiumUrl}`;
                    const placementState = placementMemory?.get(linkKey);
                    
                    if (placementState?.isPlaced && placementState.chosenLocation) {
                        // Use stored placement
                        const storedChoice = placementState.chosenLocation;
                        targetLine = storedChoice.line;
                        targetColumn = storedChoice.column;
                    } else if (searchResults.length === 1) {
                        // Auto-place single results (pre-disambiguated)
                        const singleResult = searchResults[0];
                        targetLine = singleResult.line;
                        targetColumn = singleResult.column;
                        
                        // Store the auto-placement
                        placementMemory?.set(linkKey, {
                            isPlaced: true,
                            chosenLocation: singleResult,
                            wasAmbiguous: false
                        });
                    } else {
                        // Multiple results - show disambiguation
                        const selectedResult = await showSearchDisambiguation(searchResults, parsed.regex, document);
                        if (selectedResult) {
                            targetLine = selectedResult.line;
                            targetColumn = selectedResult.column;
                            
                            // Store the choice
                            placementMemory?.set(linkKey, {
                                isPlaced: true,
                                chosenLocation: selectedResult,
                                wasAmbiguous: true
                            });
                        } else {
                            // User cancelled disambiguation
                            return null;
                        }
                    }
                }
            } catch (error) {
                debugLog(`Error during regex search: ${error}`);
                vscode.window.showErrorMessage(`Error searching for pattern "${parsed.regex}": ${error}`);
                return null;
            }
        } else if (parsed.line) {
            // Direct line navigation
            targetLine = parsed.line.startLine;
            targetColumn = parsed.line.startColumn || 1;
        }

        return { 
            range: new vscode.Range(targetLine - 1, targetColumn - 1, targetLine - 1, targetColumn - 1), 
            document 
        };

    } catch (error) {
        debugLog(`Error resolving symposium URL: ${error}`);
        vscode.window.showErrorMessage(`Failed to resolve symposium URL: ${error}`);
        return null;
    }
}

/**
 * Open a file location specified by a symposium URL
 * Full implementation with regex search support extracted from reviewWebview
 */
export async function openSymposiumUrl(
    symposiumUrl: string, 
    baseUri?: vscode.Uri,
    placementMemory?: Map<string, PlacementState>
): Promise<void> {
    // Resolve the placement
    const resolved = await resolveSymposiumUrlPlacement(symposiumUrl, baseUri, placementMemory);
    if (!resolved) {
        return; // Resolution failed or was cancelled
    }

    const { range, document } = resolved;

    // Navigate to the resolved location
    const editor = await vscode.window.showTextDocument(document, {
        selection: range,
        viewColumn: vscode.ViewColumn.One
    });

    // Add line decorations for better visibility
    const decorationRanges = createDecorationRanges(
        document, 
        undefined, // No line constraint for symposium URLs
        range.start.line + 1, // Convert back to 1-based for createDecorationRanges
        range.start.character + 1, 
        undefined // No search result highlighting needed
    );

    if (decorationRanges.length > 0) {
        const lineHighlightDecoration = vscode.window.createTextEditorDecorationType({
            backgroundColor: new vscode.ThemeColor('editor.findMatchHighlightBackground'),
            border: '1px solid',
            borderColor: new vscode.ThemeColor('editor.findMatchBorder')
        });

        editor.setDecorations(lineHighlightDecoration, decorationRanges);

        // Clear decorations after a delay
        setTimeout(() => {
            lineHighlightDecoration.dispose();
        }, 3000);
    }

    debugLog(`Successfully navigated to ${document.fileName}:${range.start.line + 1}:${range.start.character + 1}`);
}

/**
 * Show disambiguation dialog with "same as last time" option
 */
async function showSearchDisambiguationWithMemory(
    results: import('./searchEngine').SearchResult[], 
    searchTerm: string, 
    document: vscode.TextDocument,
    rememberedChoice: import('./searchEngine').SearchResult
): Promise<import('./searchEngine').SearchResult | undefined> {
    // Create "same as last time" option
    const sameAsLastItem = {
        label: `$(history) Same as last time: Line ${rememberedChoice.line}`,
        description: `${rememberedChoice.text.trim()}`,
        detail: `Column ${rememberedChoice.column} (press Enter to use this)`,
        result: rememberedChoice,
        isSameAsLast: true
    };

    // Create other options
    const otherItems = results
        .filter(r => !(r.line === rememberedChoice.line && r.column === rememberedChoice.column))
        .map((result, index) => ({
            label: `Line ${result.line}: ${result.text.trim()}`,
            description: `$(search) Match ${index + 1} of ${results.length}`,
            detail: `Column ${result.column}`,
            result: result,
            isSameAsLast: false
        }));

    const allItems = [sameAsLastItem, ...otherItems];

    const quickPick = vscode.window.createQuickPick();
    quickPick.title = `Multiple matches for "${searchTerm}"`;
    quickPick.placeholder = 'Select match (first option repeats your last choice)';
    quickPick.items = allItems;
    quickPick.canSelectMany = false;
    
    // Pre-select the "same as last time" option
    if (allItems.length > 0) {
        quickPick.activeItems = [allItems[0]];
    }

    // Create line highlight decoration type
    const lineHighlightDecoration = vscode.window.createTextEditorDecorationType({
        backgroundColor: new vscode.ThemeColor('editor.findMatchHighlightBackground'),
        border: '1px solid',
        borderColor: new vscode.ThemeColor('editor.findMatchBorder')
    });

    return new Promise((resolve) => {
        let currentActiveItem: any = null;
        let isResolved = false;

        // Show live preview as user navigates through options
        quickPick.onDidChangeActive((items) => {
            if (items.length > 0) {
                currentActiveItem = items[0];
                const selectedResult = (items[0] as any).result;
                
                // Show preview
                vscode.window.showTextDocument(document, {
                    selection: new vscode.Range(
                        selectedResult.line - 1, 
                        selectedResult.matchStart,
                        selectedResult.line - 1, 
                        selectedResult.matchEnd
                    ),
                    preview: true,
                    preserveFocus: true,
                    viewColumn: vscode.ViewColumn.One
                }).then((editor) => {
                    const decorationRanges = createDecorationRanges(
                        document, undefined, selectedResult.line, selectedResult.column, selectedResult
                    );
                    if (decorationRanges.length > 0) {
                        editor.setDecorations(lineHighlightDecoration, decorationRanges);
                        setTimeout(() => {
                            if (editor && !editor.document.isClosed) {
                                editor.setDecorations(lineHighlightDecoration, []);
                            }
                        }, 2000);
                    }
                });
            }
        });

        quickPick.onDidAccept(() => {
            if (isResolved) return;

            const selected = currentActiveItem || quickPick.selectedItems[0];
            
            if (selected && (selected as any).result) {
                const result = (selected as any).result;
                isResolved = true;
                quickPick.dispose();
                lineHighlightDecoration.dispose();
                resolve(result);
                return;
            }

            isResolved = true;
            quickPick.dispose();
            lineHighlightDecoration.dispose();
            resolve(undefined);
        });

        quickPick.onDidHide(() => {
            if (!isResolved) {
                isResolved = true;
                quickPick.dispose();
                lineHighlightDecoration.dispose();
                resolve(undefined);
            }
        });

        quickPick.show();
    });
}

/**
 * Show disambiguation dialog for multiple search results
 * Full implementation with live preview and highlighting
 */
async function showSearchDisambiguation(
    results: import('./searchEngine').SearchResult[], 
    searchTerm: string, 
    document: vscode.TextDocument
): Promise<import('./searchEngine').SearchResult | undefined> {
    // Create QuickPick items with context
    const items = results.map((result, index) => ({
        label: `Line ${result.line}: ${result.text.trim()}`,
        description: `$(search) Match ${index + 1} of ${results.length}`,
        detail: `Column ${result.column}`,
        result: result
    }));

    const quickPick = vscode.window.createQuickPick();
    quickPick.title = `Multiple matches for "${searchTerm}"`;
    quickPick.placeholder = 'Select the match you want to navigate to (preview updates as you navigate)';
    quickPick.items = items;
    quickPick.canSelectMany = false;

    // Create line highlight decoration type
    const lineHighlightDecoration = vscode.window.createTextEditorDecorationType({
        backgroundColor: new vscode.ThemeColor('editor.findMatchHighlightBackground'),
        border: '1px solid',
        borderColor: new vscode.ThemeColor('editor.findMatchBorder')
    });

    return new Promise((resolve) => {
        let currentActiveItem: any = null;
        let isResolved = false;

        // Show live preview as user navigates through options
        quickPick.onDidChangeActive((items) => {
            if (items.length > 0) {
                currentActiveItem = items[0]; // Track the currently active item
                const selectedResult = (items[0] as any).result;
                
                // Show preview by revealing the location without committing to it
                vscode.window.showTextDocument(document, {
                    selection: new vscode.Range(
                        selectedResult.line - 1, 
                        selectedResult.matchStart,
                        selectedResult.line - 1, 
                        selectedResult.matchEnd
                    ),
                    preview: true, // This keeps it as a preview tab
                    preserveFocus: true, // Keep focus on the QuickPick
                    viewColumn: vscode.ViewColumn.One // Ensure it opens in main editor area
                }).then((editor) => {
                    // Add line decorations to preview just like final navigation
                    const decorationRanges = createDecorationRanges(
                        document, 
                        undefined, // No line constraint for search results
                        selectedResult.line, 
                        selectedResult.column, 
                        selectedResult
                    );
                    if (decorationRanges.length > 0) {
                        editor.setDecorations(lineHighlightDecoration, decorationRanges);
                        
                        // Remove preview highlight after 2 seconds (shorter than final)
                        setTimeout(() => {
                            if (editor && !editor.document.isClosed) {
                                editor.setDecorations(lineHighlightDecoration, []);
                            }
                        }, 2000);
                    }
                }, (error: any) => {
                    console.log(`Preview failed: ${error}`);
                });
            }
        });

        quickPick.onDidAccept(() => {
            if (isResolved) {
                return;
            }

            // Use the currently active item instead of selectedItems
            const selected = currentActiveItem || quickPick.selectedItems[0];
            
            if (selected && (selected as any).result) {
                const result = (selected as any).result;
                isResolved = true;
                quickPick.dispose();
                lineHighlightDecoration.dispose();
                resolve(result);
                return;
            }

            // Fallback case
            isResolved = true;
            quickPick.dispose();
            lineHighlightDecoration.dispose();
            resolve(undefined);
        });

        quickPick.onDidHide(() => {
            if (!isResolved) {
                isResolved = true;
                quickPick.dispose();
                lineHighlightDecoration.dispose();
                resolve(undefined);
            }
        });

        quickPick.show();
    });
}

// clearChoiceMemory is no longer needed - placement memory is managed by WalkthroughWebviewProvider

/**
 * Create decoration ranges based on line specification or search result
 */
function createDecorationRanges(
    document: vscode.TextDocument, 
    lineSpec?: import('./symposiumUrl').LineSpec, 
    targetLine?: number, 
    targetColumn?: number,
    searchResult?: import('./searchEngine').SearchResult
): vscode.Range[] {
    // If we have a search result, highlight the exact match
    if (searchResult) {
        const line = Math.max(0, searchResult.line - 1); // Convert to 0-based
        const startCol = searchResult.matchStart;
        const endCol = searchResult.matchEnd;
        return [new vscode.Range(line, startCol, line, endCol)];
    }
    
    if (lineSpec) {
        const ranges: vscode.Range[] = [];
        const startLine = Math.max(0, lineSpec.startLine - 1);
        const endLine = lineSpec.endLine ? Math.max(0, lineSpec.endLine - 1) : startLine;
        
        for (let line = startLine; line <= Math.min(endLine, document.lineCount - 1); line++) {
            const lineText = document.lineAt(line);
            ranges.push(new vscode.Range(line, 0, line, lineText.text.length));
        }
        return ranges;
    }
    
    // Single line highlight
    if (targetLine) {
        const line = Math.max(0, targetLine - 1);
        if (line < document.lineCount) {
            const lineText = document.lineAt(line);
            return [new vscode.Range(line, 0, line, lineText.text.length)];
        }
    }
    
    return [];
}
