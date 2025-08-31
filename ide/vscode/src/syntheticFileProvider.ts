import * as vscode from 'vscode';

/**
 * Provides virtual file content for synthetic PR diffs.
 * Supports sshell-original:// and sshell-modified:// schemes.
 */
export class SyntheticFileProvider implements vscode.FileSystemProvider {
    private files = new Map<string, Uint8Array>();
    private _emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
    readonly onDidChangeFile = this._emitter.event;

    /**
     * Store virtual file content for diff display
     */
    setFileContent(uri: vscode.Uri, content: string): void {
        this.files.set(uri.toString(), Buffer.from(content, 'utf8'));
    }

    // FileSystemProvider implementation
    readFile(uri: vscode.Uri): Uint8Array {
        const content = this.files.get(uri.toString());
        if (!content) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
        return content;
    }

    stat(uri: vscode.Uri): vscode.FileStat {
        const content = this.files.get(uri.toString());
        if (!content) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
        return {
            type: vscode.FileType.File,
            ctime: Date.now(),
            mtime: Date.now(),
            size: content.length
        };
    }

    // Required but unused for our read-only use case
    writeFile(): void { throw vscode.FileSystemError.NoPermissions(); }
    delete(): void { throw vscode.FileSystemError.NoPermissions(); }
    rename(): void { throw vscode.FileSystemError.NoPermissions(); }
    createDirectory(): void { throw vscode.FileSystemError.NoPermissions(); }
    readDirectory(): [string, vscode.FileType][] { return []; }
    watch(): vscode.Disposable { return new vscode.Disposable(() => {}); }
}
