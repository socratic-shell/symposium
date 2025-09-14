import * as vscode from 'vscode';

interface SyntheticPRData {
    review_id: string;
    title: string;
    description: any;
    commit_range: string;
    files_changed: FileChange[];
    comment_threads: CommentThread[];
    status: string;
}

interface FileChange {
    path: string;
    additions: number;
    deletions: number;
    hunks: any[];
}

interface CommentThread {
    id: string;
    file_path: string;
    line_number: number;
    comment_type: 'insight' | 'question' | 'todo' | 'fixme';
    content: string;
}

class PRTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly itemType: 'pr' | 'files' | 'file' | 'comments' | 'comment' | 'actions' | 'action' | 'placeholder',
        public readonly data?: any
    ) {
        super(label, collapsibleState);
        this.contextValue = itemType;
        
        if (itemType === 'file') {
            // Use diff command instead of opening file normally
            this.command = {
                command: 'socratic-shell.showFileDiff',
                title: 'Show Diff',
                arguments: [data.path]
            };
        } else if (itemType === 'comment') {
            this.command = {
                command: 'vscode.open',
                title: 'Go to Comment',
                arguments: [
                    vscode.Uri.file(data.file_path),
                    { selection: new vscode.Range(data.line_number - 1, 0, data.line_number - 1, 0) }
                ]
            };
        } else if (itemType === 'action') {
            // Add command for action buttons
            this.command = {
                command: 'socratic-shell.reviewAction',
                title: 'Review Action',
                arguments: [data.action]
            };
        }
    }
}

export class SyntheticPRTreeProvider implements vscode.TreeDataProvider<PRTreeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<PRTreeItem | undefined | null | void> = new vscode.EventEmitter<PRTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<PRTreeItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private currentPR: SyntheticPRData | null = null;
    private commentsExpanded: boolean = true;

    constructor() {}

    toggleCommentsExpansion(): void {
        this.commentsExpanded = !this.commentsExpanded;
        this.refresh();
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    updatePR(prData: SyntheticPRData): void {
        console.log('[TREE PROVIDER] updatePR called with:', prData.title);
        this.currentPR = prData;
        this.refresh();
    }

    clearPR(): void {
        this.currentPR = null;
        this.refresh();
    }

    getTreeItem(element: PRTreeItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: PRTreeItem): Thenable<PRTreeItem[]> {
        console.log('[TREE PROVIDER] getChildren called, currentPR:', !!this.currentPR, 'element:', element?.itemType);
        
        if (!this.currentPR) {
            console.log('[TREE PROVIDER] No current PR, showing placeholder');
            return Promise.resolve([
                new PRTreeItem(
                    'No active pull request',
                    vscode.TreeItemCollapsibleState.None,
                    'placeholder'
                )
            ]);
        }

        if (!element) {
            // Root level - show PR title
            return Promise.resolve([
                new PRTreeItem(
                    `${this.currentPR.title} (${this.currentPR.commit_range})`,
                    vscode.TreeItemCollapsibleState.Expanded,
                    'pr'
                )
            ]);
        }

        if (element.itemType === 'pr') {
            // PR children - Files, Comments, Actions
            return Promise.resolve([
                new PRTreeItem(
                    `Files Changed (${this.currentPR.files_changed.length})`,
                    vscode.TreeItemCollapsibleState.Expanded,
                    'files'
                ),
                new PRTreeItem(
                    `Comments (${this.currentPR.comment_threads.length})`,
                    this.commentsExpanded ? vscode.TreeItemCollapsibleState.Expanded : vscode.TreeItemCollapsibleState.Collapsed,
                    'comments'
                ),
                new PRTreeItem(
                    'Actions',
                    vscode.TreeItemCollapsibleState.Expanded,
                    'actions'
                )
            ]);
        }

        if (element.itemType === 'files') {
            // Show individual files with comment indicators
            if (!this.currentPR) return Promise.resolve([]);
            
            return Promise.resolve(
                this.currentPR.files_changed.map(file => {
                    const commentsInFile = this.currentPR!.comment_threads.filter(c => c.file_path === file.path);
                    const commentIndicator = commentsInFile.length > 0 ? ` 💬${commentsInFile.length}` : '';
                    
                    return new PRTreeItem(
                        `${file.path} (+${file.additions} -${file.deletions})${commentIndicator}`,
                        vscode.TreeItemCollapsibleState.None,
                        'file',
                        file
                    );
                })
            );
        }

        if (element.itemType === 'comments') {
            // Show individual comments
            return Promise.resolve(
                this.currentPR.comment_threads.map(comment => 
                    new PRTreeItem(
                        `${this.getCommentIcon(comment.comment_type)} ${comment.file_path}:${comment.line_number}`,
                        vscode.TreeItemCollapsibleState.None,
                        'comment',
                        comment
                    )
                )
            );
        }

        if (element.itemType === 'actions') {
            // Show feedback actions
            return Promise.resolve([
                new PRTreeItem(
                    '✅ Request Changes',
                    vscode.TreeItemCollapsibleState.None,
                    'action',
                    { action: 'request_changes' }
                ),
                new PRTreeItem(
                    '📝 Checkpoint Work',
                    vscode.TreeItemCollapsibleState.None,
                    'action',
                    { action: 'checkpoint' }
                ),
                new PRTreeItem(
                    '↩️ Close Review',
                    vscode.TreeItemCollapsibleState.None,
                    'action',
                    { action: 'return' }
                )
            ]);
        }

        return Promise.resolve([]);
    }

    private getCommentIcon(type: string): string {
        switch (type) {
            case 'insight': return '💡';
            case 'question': return '❓';
            case 'todo': return '📝';
            case 'fixme': return '🔧';
            default: return '💬';
        }
    }
}
