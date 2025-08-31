import * as vscode from 'vscode';

export class ReviewProvider implements vscode.TreeDataProvider<ReviewItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<ReviewItem | undefined | null | void> = new vscode.EventEmitter<ReviewItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<ReviewItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private reviewContent: string = '';
    private reviewItems: ReviewItem[] = [];

    constructor() {
        this.loadDummyReview();
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: ReviewItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: ReviewItem): Thenable<ReviewItem[]> {
        if (!element) {
            return Promise.resolve(this.reviewItems);
        }
        return Promise.resolve(element.children || []);
    }

    showDummyReview(): void {
        this.loadDummyReview();
        this.refresh();
    }

    copyReviewToClipboard(): void {
        vscode.env.clipboard.writeText(this.reviewContent).then(() => {
            vscode.window.showInformationMessage('Review copied to clipboard!');
        });
    }

    // 💡: Update review content from MCP server via IPC
    updateReview(content: string, mode: 'replace' | 'update-section' | 'append' = 'replace', section?: string): void {
        switch (mode) {
            case 'replace':
                this.reviewContent = content;
                break;
            case 'append':
                this.reviewContent += '\n\n' + content;
                break;
            case 'update-section':
                if (section) {
                    // 💡: For MVP, just append with section header
                    // Future enhancement could implement smart section replacement
                    this.reviewContent += `\n\n## ${section}\n${content}`;
                } else {
                    // Fallback to append if no section specified
                    this.reviewContent += '\n\n' + content;
                }
                break;
        }
        
        // 💡: Parse the updated content and refresh the tree view
        this.reviewItems = this.parseMarkdownToTree(this.reviewContent);
        this.refresh();
        
        console.log('Review updated via IPC:', mode, section ? `(section: ${section})` : '');
    }

    private loadDummyReview(): void {
        this.reviewContent = `# Add user authentication system

## Context
The application needed secure user authentication to protect user data and enable personalized features. This implements a JWT-based authentication system with secure password hashing.

## Changes Made
- Added authentication middleware (src/auth/middleware.ts:23)
- Created user login/signup endpoints (src/routes/auth.ts:45) 
- Updated user model with password hashing (src/models/user.ts:67)
- Added JWT token generation and validation (src/utils/jwt.ts:12)

## Implementation Details

### Authentication Flow (src/auth/middleware.ts:23)
The middleware intercepts requests and validates JWT tokens. If the token is valid, the user object is attached to the request for downstream handlers to use.

### Password Security (src/models/user.ts:67)
Passwords are hashed using bcrypt with a salt factor of 12. The plaintext password is never stored in the database.

## Design Decisions
- Used JWT tokens for stateless authentication
- Chose bcrypt over other hashing algorithms for better security
- Token expiration set to 24 hours for balance of security and UX`;

        this.reviewItems = this.parseMarkdownToTree(this.reviewContent);
    }

    private parseMarkdownToTree(markdown: string): ReviewItem[] {
        const lines = markdown.split('\n');
        const items: ReviewItem[] = [];
        let currentSection: ReviewItem | null = null;

        for (const line of lines) {
            if (line.startsWith('# ')) {
                const item = new ReviewItem(
                    line.substring(2),
                    vscode.TreeItemCollapsibleState.Expanded,
                    'title'
                );
                items.push(item);
                currentSection = item;
            } else if (line.startsWith('## ')) {
                const item = new ReviewItem(
                    line.substring(3),
                    vscode.TreeItemCollapsibleState.Expanded,
                    'section'
                );
                items.push(item);
                currentSection = item;
            } else if (line.startsWith('### ')) {
                const item = new ReviewItem(
                    line.substring(4),
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'subsection'
                );
                if (currentSection) {
                    if (!currentSection.children) {
                        currentSection.children = [];
                    }
                    currentSection.children.push(item);
                }
            } else if (line.trim().startsWith('- ')) {
                const content = line.trim().substring(2);
                const item = this.createContentItem(content);
                if (currentSection) {
                    if (!currentSection.children) {
                        currentSection.children = [];
                    }
                    currentSection.children.push(item);
                }
            } else if (line.trim() && !line.startsWith('#')) {
                const item = this.createContentItem(line.trim());
                if (currentSection) {
                    if (!currentSection.children) {
                        currentSection.children = [];
                    }
                    currentSection.children.push(item);
                }
            }
        }

        return items;
    }

    private createContentItem(content: string): ReviewItem {
        // Check for file:line references
        const fileRefMatch = content.match(/\(([^:)]+):(\d+)\)/);
        
        const item = new ReviewItem(
            content,
            vscode.TreeItemCollapsibleState.None,
            'content'
        );

        if (fileRefMatch) {
            const fileName = fileRefMatch[1];
            const lineNumber = parseInt(fileRefMatch[2]) - 1; // VSCode uses 0-based line numbers
            
            // Make it clickable by adding a command
            item.command = {
                command: 'vscode.open',
                title: 'Open File',
                arguments: [
                    vscode.Uri.file(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath + '/' + fileName),
                    {
                        selection: new vscode.Range(lineNumber, 0, lineNumber, 0)
                    }
                ]
            };
            
            item.tooltip = `Click to open ${fileName}:${lineNumber + 1}`;
        }

        return item;
    }
}

class ReviewItem extends vscode.TreeItem {
    public children?: ReviewItem[];
    public itemType: 'title' | 'section' | 'subsection' | 'content';

    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        itemType: 'title' | 'section' | 'subsection' | 'content'
    ) {
        super(label, collapsibleState);

        this.itemType = itemType;
        this.tooltip = this.label;
        
        // Set different icons based on item type
        switch (itemType) {
            case 'title':
                this.iconPath = new vscode.ThemeIcon('file-text');
                break;
            case 'section':
                this.iconPath = new vscode.ThemeIcon('symbol-namespace');
                break;
            case 'subsection':
                this.iconPath = new vscode.ThemeIcon('symbol-method');
                break;
            case 'content':
                this.iconPath = new vscode.ThemeIcon('symbol-string');
                break;
        }
    }
}