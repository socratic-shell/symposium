"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SyntheticFileProvider = void 0;
const vscode = require("vscode");
/**
 * Provides virtual file content for synthetic PR diffs.
 * Supports sshell-original:// and sshell-modified:// schemes.
 */
class SyntheticFileProvider {
    constructor() {
        this.files = new Map();
        this._emitter = new vscode.EventEmitter();
        this.onDidChangeFile = this._emitter.event;
    }
    /**
     * Store virtual file content for diff display
     */
    setFileContent(uri, content) {
        this.files.set(uri.toString(), Buffer.from(content, 'utf8'));
    }
    // FileSystemProvider implementation
    readFile(uri) {
        const content = this.files.get(uri.toString());
        if (!content) {
            throw vscode.FileSystemError.FileNotFound(uri);
        }
        return content;
    }
    stat(uri) {
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
    writeFile() { throw vscode.FileSystemError.NoPermissions(); }
    delete() { throw vscode.FileSystemError.NoPermissions(); }
    rename() { throw vscode.FileSystemError.NoPermissions(); }
    createDirectory() { throw vscode.FileSystemError.NoPermissions(); }
    readDirectory() { return []; }
    watch() { return new vscode.Disposable(() => { }); }
}
exports.SyntheticFileProvider = SyntheticFileProvider;
//# sourceMappingURL=syntheticFileProvider.js.map