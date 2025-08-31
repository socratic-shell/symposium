"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SyntheticPRTreeProvider = void 0;
const vscode = require("vscode");
class PRTreeItem extends vscode.TreeItem {
    constructor(label, collapsibleState, itemType, data) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.itemType = itemType;
        this.data = data;
        this.contextValue = itemType;
        if (itemType === 'file') {
            // Use diff command instead of opening file normally
            this.command = {
                command: 'symposium.showFileDiff',
                title: 'Show Diff',
                arguments: [data.path]
            };
        }
        else if (itemType === 'comment') {
            this.command = {
                command: 'vscode.open',
                title: 'Go to Comment',
                arguments: [
                    vscode.Uri.file(data.file_path),
                    { selection: new vscode.Range(data.line_number - 1, 0, data.line_number - 1, 0) }
                ]
            };
        }
        else if (itemType === 'action') {
            // Add command for action buttons
            this.command = {
                command: 'symposium.reviewAction',
                title: 'Review Action',
                arguments: [data.action]
            };
        }
    }
}
class SyntheticPRTreeProvider {
    constructor() {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.currentPR = null;
        this.commentsExpanded = true;
    }
    toggleCommentsExpansion() {
        this.commentsExpanded = !this.commentsExpanded;
        this.refresh();
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    updatePR(prData) {
        console.log('[TREE PROVIDER] updatePR called with:', prData.title);
        this.currentPR = prData;
        this.refresh();
    }
    clearPR() {
        this.currentPR = null;
        this.refresh();
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        console.log('[TREE PROVIDER] getChildren called, currentPR:', !!this.currentPR, 'element:', element?.itemType);
        if (!this.currentPR) {
            console.log('[TREE PROVIDER] No current PR, showing placeholder');
            return Promise.resolve([
                new PRTreeItem('No active pull request', vscode.TreeItemCollapsibleState.None, 'placeholder')
            ]);
        }
        if (!element) {
            // Root level - show PR title
            return Promise.resolve([
                new PRTreeItem(`${this.currentPR.title} (${this.currentPR.commit_range})`, vscode.TreeItemCollapsibleState.Expanded, 'pr')
            ]);
        }
        if (element.itemType === 'pr') {
            // PR children - Files, Comments, Actions
            return Promise.resolve([
                new PRTreeItem(`Files Changed (${this.currentPR.files_changed.length})`, vscode.TreeItemCollapsibleState.Expanded, 'files'),
                new PRTreeItem(`Comments (${this.currentPR.comment_threads.length})`, this.commentsExpanded ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.Collapsed, 'comments'),
                new PRTreeItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'actions')
            ]);
        }
        if (element.itemType === 'files') {
            // Show individual files with comment indicators
            if (!this.currentPR)
                return Promise.resolve([]);
            return Promise.resolve(this.currentPR.files_changed.map(file => {
                const commentsInFile = this.currentPR.comment_threads.filter(c => c.file_path === file.path);
                const commentIndicator = commentsInFile.length > 0 ? ` 💬${commentsInFile.length}` : '';
                return new PRTreeItem(`${file.path} (+${file.additions} -${file.deletions})${commentIndicator}`, vscode.TreeItemCollapsibleState.None, 'file', file);
            }));
        }
        if (element.itemType === 'comments') {
            // Show individual comments
            return Promise.resolve(this.currentPR.comment_threads.map(comment => new PRTreeItem(`${this.getCommentIcon(comment.comment_type)} ${comment.file_path}:${comment.line_number}`, vscode.TreeItemCollapsibleState.None, 'comment', comment)));
        }
        if (element.itemType === 'actions') {
            // Show feedback actions
            return Promise.resolve([
                new PRTreeItem('✅ Request Changes', vscode.TreeItemCollapsibleState.None, 'action', { action: 'request_changes' }),
                new PRTreeItem('📝 Checkpoint Work', vscode.TreeItemCollapsibleState.None, 'action', { action: 'checkpoint' }),
                new PRTreeItem('↩️ Close Review', vscode.TreeItemCollapsibleState.None, 'action', { action: 'return' })
            ]);
        }
        return Promise.resolve([]);
    }
    getCommentIcon(type) {
        switch (type) {
            case 'insight': return '💡';
            case 'question': return '❓';
            case 'todo': return '📝';
            case 'fixme': return '🔧';
            default: return '💬';
        }
    }
}
exports.SyntheticPRTreeProvider = SyntheticPRTreeProvider;
//# sourceMappingURL=syntheticPRTreeProvider.js.map