# VS Code Pull Request Provider Extension API Research

## Overview

This document provides a comprehensive guide to implementing a VS Code extension that creates synthetic pull requests for LLM-generated code changes, using VS Code's existing Pull Request Provider APIs and related extension interfaces.

## Core Architectural Concepts

### Pull Request Provider Pattern

VS Code's pull request experience is powered by extension APIs that allow extension authors to create extensions that manage pull requests and their related metadata. This open extension model means that pull request providers work just like existing source control providers and anyone can write an extension for VS Code that provides in-editor commenting and capabilities to review source code hosted on their platform.

The key insight is that VS Code extensions are meant to be separate for each Pull Request provider rather than trying to make one extension integrate with multiple git providers, which would make it bloated.

## Key Extension APIs Required

### 1. Comment Controller API

The Comment Controller API is the foundation for creating PR-like commenting experiences.

#### Basic Comment Controller Setup

```typescript
import * as vscode from 'vscode';

// Create a comment controller - this manages all commenting for your extension
const commentController = vscode.comments.createCommentController(
    'synthetic-pr', 
    'Synthetic Pull Request'
);

// Register it for cleanup
context.subscriptions.push(commentController);

// Set up commenting range provider to control where comments can be added
commentController.commentingRangeProvider = {
    provideCommentingRanges: (document: vscode.TextDocument) => {
        // Return ranges where commenting is allowed
        return [new vscode.Range(0, 0, document.lineCount - 1, 0)];
    }
};
```

#### Comment Thread Management

Comment threads represent conversations at particular ranges in a document.

```typescript
// Create a comment thread
const thread = commentController.createCommentThread(
    document.uri,           // URI of the document
    new vscode.Range(5, 0, 5, 10), // Range where comment appears
    []                      // Initial comments (empty for new thread)
);

// Configure the thread
thread.label = 'LLM Explanation';
thread.canReply = true;
thread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded;
```

#### Comment Implementation

Comments must implement the vscode.Comment interface.

```typescript
class LLMComment implements vscode.Comment {
    id: number;
    label: string | undefined;
    savedBody: string | vscode.MarkdownString;
    
    constructor(
        public body: string | vscode.MarkdownString,
        public mode: vscode.CommentMode,
        public author: vscode.CommentAuthorInformation,
        public parent?: vscode.CommentThread,
        public contextValue?: string
    ) {
        this.id = ++commentId;
        this.savedBody = this.body;
    }
}

// Add pre-populated LLM explanations
const explanation = new LLMComment(
    "This change implements the new authentication flow as requested. The method validates the token before proceeding with the API call.",
    vscode.CommentMode.Preview,
    { name: 'AI Assistant', iconPath: aiIconUri },
    thread,
    'llm-explanation'
);

thread.comments = [explanation];
```

#### Comment Commands and Reactions

Extensions can register reaction handlers for creating and deleting reactions on comments.

```typescript
// Register reaction handler
commentController.reactionHandler = async (comment: vscode.Comment, reaction: vscode.CommentReaction) => {
    // Handle thumbs up/down, approval, request changes etc.
    if (reaction.label === '👍') {
        // Mark as approved
    } else if (reaction.label === '❓') {
        // Request clarification from LLM
        await requestLLMClarification(comment);
    }
};

// Register commands for comment actions
context.subscriptions.push(
    vscode.commands.registerCommand('synthetic-pr.requestChanges', (comment: LLMComment) => {
        // Trigger LLM to make changes based on comment
        triggerLLMModification(comment);
    })
);
```

### 2. Tree View API for PR Navigation

The Tree View API allows you to create hierarchical views in VS Code's sidebar.

#### Tree Data Provider Implementation

```typescript
interface PRItem {
    label: string;
    type: 'pr' | 'file' | 'change';
    uri?: vscode.Uri;
    children?: PRItem[];
    description?: string;
}

class PRTreeDataProvider implements vscode.TreeDataProvider<PRItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<PRItem | undefined | null | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private workspaceRoot: string) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: PRItem): vscode.TreeItem {
        const item = new vscode.TreeItem(element.label);
        
        if (element.type === 'pr') {
            item.tooltip = `Synthetic PR: ${element.label}`;
            item.collapsibleState = vscode.TreeItemCollapsibleState.Expanded;
            item.iconPath = new vscode.ThemeIcon('git-pull-request');
        } else if (element.type === 'file') {
            item.collapsibleState = vscode.TreeItemCollapsibleState.Collapsed;
            item.command = {
                command: 'synthetic-pr.openDiff',
                title: 'Open Diff',
                arguments: [element.uri]
            };
            item.iconPath = vscode.ThemeIcon.File;
        }
        
        item.contextValue = element.type;
        return item;
    }

    async getChildren(element?: PRItem): Promise<PRItem[]> {
        if (!element) {
            // Return root level - active synthetic PRs
            return this.getSyntheticPRs();
        }
        
        if (element.type === 'pr') {
            // Return changed files for this PR
            return this.getChangedFiles(element);
        }
        
        return element.children || [];
    }

    private async getSyntheticPRs(): Promise<PRItem[]> {
        // Return list of active synthetic PRs
        return [
            {
                label: 'AI Assistant Changes',
                type: 'pr',
                description: 'Generated 2 hours ago',
                children: []
            }
        ];
    }

    private async getChangedFiles(pr: PRItem): Promise<PRItem[]> {
        // Return files changed in this PR with diff stats
        return [
            {
                label: 'src/auth.ts',
                type: 'file',
                uri: vscode.Uri.file('src/auth.ts'),
                description: '+15 -3'
            },
            {
                label: 'src/api.ts', 
                type: 'file',
                uri: vscode.Uri.file('src/api.ts'),
                description: '+8 -1'
            }
        ];
    }
}

// Register the tree view
const prTreeProvider = new PRTreeDataProvider(workspaceRoot);
vscode.window.registerTreeDataProvider('syntheticPRs', prTreeProvider);

// Or create tree view for more control
const prTreeView = vscode.window.createTreeView('syntheticPRs', {
    treeDataProvider: prTreeProvider,
    showCollapseAll: true
});
```

#### Tree View Contribution in package.json

```json
{
    "contributes": {
        "views": {
            "scm": [
                {
                    "id": "syntheticPRs",
                    "name": "Synthetic Pull Requests",
                    "when": "workspaceFolderCount > 0"
                }
            ]
        },
        "commands": [
            {
                "command": "synthetic-pr.refresh",
                "title": "Refresh",
                "icon": "$(refresh)"
            },
            {
                "command": "synthetic-pr.openDiff",
                "title": "Open Diff"
            }
        ],
        "menus": {
            "view/title": [
                {
                    "command": "synthetic-pr.refresh",
                    "when": "view == syntheticPRs",
                    "group": "navigation"
                }
            ],
            "view/item/context": [
                {
                    "command": "synthetic-pr.approvePR",
                    "when": "view == syntheticPRs && viewItem == pr",
                    "group": "inline"
                }
            ]
        }
    }
}
```

### 3. Webview API for PR Details Panel

Webviews allow you to create fully customizable views within VS Code using HTML, CSS, and JavaScript. The GitHub Pull Requests extension uses webviews for detailed PR views.

#### Basic Webview Panel

```typescript
function createPRDetailsPanel(context: vscode.ExtensionContext, prData: any) {
    const panel = vscode.window.createWebviewPanel(
        'syntheticPRDetails',
        'Pull Request Details',
        vscode.ViewColumn.One,
        {
            enableScripts: true,
            retainContextWhenHidden: true,
            localResourceRoots: [
                vscode.Uri.joinPath(context.extensionUri, 'media'),
                vscode.Uri.joinPath(context.extensionUri, 'out')
            ]
        }
    );

    // Set webview content
    panel.webview.html = getPRDetailsHTML(panel.webview, context.extensionUri, prData);

    // Handle messages from webview
    panel.webview.onDidReceiveMessage(
        message => {
            switch (message.command) {
                case 'approve':
                    handlePRApproval(prData);
                    break;
                case 'requestChanges':
                    handleRequestChanges(message.feedback);
                    break;
                case 'addComment':
                    addInlineComment(message.file, message.line, message.comment);
                    break;
            }
        },
        undefined,
        context.subscriptions
    );

    return panel;
}
```

#### Webview Content Generation

```typescript
function getPRDetailsHTML(webview: vscode.Webview, extensionUri: vscode.Uri, prData: any): string {
    // Get URIs for static resources
    const stylesUri = webview.asWebviewUri(
        vscode.Uri.joinPath(extensionUri, 'media', 'pr-details.css')
    );
    const scriptUri = webview.asWebviewUri(
        vscode.Uri.joinPath(extensionUri, 'out', 'pr-details.js')
    );

    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link href="${stylesUri}" rel="stylesheet">
    <title>PR Details</title>
</head>
<body>
    <div class="pr-header">
        <h1>${prData.title}</h1>
        <div class="pr-meta">
            <span class="author">By: ${prData.author}</span>
            <span class="timestamp">${prData.timestamp}</span>
        </div>
    </div>
    
    <div class="pr-description">
        <h2>AI Assistant Summary</h2>
        <p>${prData.description}</p>
    </div>
    
    <div class="pr-actions">
        <button id="approve-btn" class="btn btn-success">✓ Approve</button>
        <button id="request-changes-btn" class="btn btn-warning">⚠ Request Changes</button>
        <button id="merge-btn" class="btn btn-primary">Merge</button>
    </div>
    
    <div class="file-changes">
        <h2>Files Changed (${prData.filesChanged.length})</h2>
        ${prData.filesChanged.map(file => `
            <div class="file-item" data-file="${file.path}">
                <div class="file-header">
                    <span class="file-name">${file.path}</span>
                    <span class="diff-stats">+${file.additions} -${file.deletions}</span>
                </div>
            </div>
        `).join('')}
    </div>
    
    <script>
        const vscode = acquireVsCodeApi();
        
        document.getElementById('approve-btn').addEventListener('click', () => {
            vscode.postMessage({ command: 'approve' });
        });
        
        document.getElementById('request-changes-btn').addEventListener('click', () => {
            const feedback = prompt('What changes would you like to request?');
            if (feedback) {
                vsc