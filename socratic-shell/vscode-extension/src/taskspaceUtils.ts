import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Detect the current taskspace UUID by walking up the directory tree
 * looking for a directory matching the task-{uuid} pattern with a taskspace.json file
 */
export function getCurrentTaskspaceUuid(): string | null {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath;
    const taskUuidPattern = /^task-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i;

    let currentDir = workspaceRoot;
    while (currentDir !== path.dirname(currentDir)) {
        const dirName = path.basename(currentDir);
        const match = dirName.match(taskUuidPattern);

        if (match) {
            const taskspaceJsonPath = path.join(currentDir, 'taskspace.json');
            if (fs.existsSync(taskspaceJsonPath)) {
                return match[1];
            }
        }

        currentDir = path.dirname(currentDir);
    }

    return null;
}
