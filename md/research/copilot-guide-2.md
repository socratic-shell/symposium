# How Visual Diff Highlighting and File Change Visualization Work in the VS Code GitHub PR Extension

## **Goal**
Replicate the user experience of GitHub-style PR review in VS Code for local changes—not just for real PRs, but for arbitrary sets of file diffs (e.g., between `HEAD^`, `HEAD`, staged, and unstaged changes), including:
- Red/green diff highlighting
- File tree view of changed files
- Commenting on diffs (with custom backend logic)

---

## **How the VS Code Extension Implements Diff Highlighting**

### **1. Diff Editor & Red/Green Highlighting**

#### **VS Code Built-In Functionality**
- The VS Code diff editor is responsible for rendering the red (deletions) and green (additions) highlights.
- You can open a diff editor using the API:
  ```typescript
  vscode.commands.executeCommand('vscode.diff', originalUri, modifiedUri, title)
  ```
- **You do not need to manually paint the highlights.** You just provide the original and modified file contents (could be real, or virtual documents), and VS Code’s diff engine does the rest.

#### **Extension Example**
- The extension uses custom URI schemes and [TextDocumentContentProvider](https://code.visualstudio.com/api/extension-guides/virtual-documents) to provide virtual file versions for diffing.
- See this logic in [`GitFileChangeModel`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/fileChangeModel.ts) and [how URIs are created](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/common/uri.ts).

#### **Relevant Code**
- Opening diffs:  
  [`reviewManager.ts: openDiff()`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewManager.ts#L608-L624)
- Providing file contents for virtual docs:  
  [`reviewManager.ts: provideTextDocumentContent()`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewManager.ts#L1248-L1295)
- Custom URI construction:  
  [`common/uri.ts: toReviewUri()`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/common/uri.ts)

---

### **2. File Tree/List of Changed Files**

- The extension uses a [TreeDataProvider](https://code.visualstudio.com/api/extension-guides/tree-view) to show changed files grouped by folder, status, etc.
- Each node in the tree can be clicked to open the diff editor for that file.

#### **Relevant Code**
- [`prChangesTreeDataProvider.ts`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/prChangesTreeDataProvider.ts): Implements the file/folder tree for changed files in a PR.
- [`GitFileChangeNode`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/treeNodes/fileChangeNode.ts): Individual nodes for changed files.

---

### **3. Comments on Diffs**

- The extension uses VS Code’s [CommentController API](https://code.visualstudio.com/api/extension-guides/comments) to display comment threads in diff editors.
- Comments are mapped to lines/hunks in virtual diff editors using custom logic.

#### **Relevant Code**
- [`reviewCommentController.ts`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewCommentController.ts): Handles in-diff comment thread logic.
- [`pullRequestCommentController.ts`](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/pullRequestCommentController.ts): Integrates comments into the PR UI.

---

## **What Is Provided by VS Code, and What Is Custom?**

| Feature                | VS Code Provides                        | Extension Implements ("Open-Coded")      |
|------------------------|-----------------------------------------|------------------------------------------|
| Diff editor UI         | Yes (automatic)                         | Provides URIs and file contents          |
| Red/green highlighting | Yes                                     | No manual logic needed                   |
| File list/tree         | Tree UI API, basic SCM integration      | Custom data providers, tree node logic   |
| Commenting             | Comments API, UI hooks for threads      | Custom mapping, persistence, logic       |
| Staging/unstaged diff  | SCM API, but only for HEAD/index/WD     | Can provide arbitrary hunks via provider |

---

## **How to Visualize Your Own Diff Hunks**

- Create a [TextDocumentContentProvider](https://code.visualstudio.com/api/extension-guides/virtual-documents) for a custom URI scheme.
- For each hunk, generate virtual file contents representing "original" and "modified" versions.
- Use `vscode.diff` to open a diff editor between these URIs.
- VS Code will automatically highlight changes.
- If you want to support comments, register a [CommentController](https://code.visualstudio.com/api/extension-guides/comments) for your diff editors.

**You can provide any set of contents—does not have to be from git! This allows arbitrary diffs, including staged, unstaged, or synthetic hunks.**

---

## **Should You Use Git Directly?**

- If you want to show diffs for actual commits, staged, or unstaged changes, you can use git commands (e.g., `git show`, `git diff`, etc.) to get file contents.
- VS Code’s built-in Git extension only visualizes diffs for HEAD/index/WD, not arbitrary hunks.
- For arbitrary hunks (like those generated by your app), you must use your own content provider.

---

## **Key URLs for Reference**

- [reviewManager.ts: openDiff()](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewManager.ts#L608-L624)
- [reviewManager.ts: provideTextDocumentContent()](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewManager.ts#L1248-L1295)
- [GitFileChangeModel](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/fileChangeModel.ts)
- [prChangesTreeDataProvider.ts](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/prChangesTreeDataProvider.ts)
- [reviewCommentController.ts](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewCommentController.ts)
- [pullRequestCommentController.ts](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/pullRequestCommentController.ts)
- [TextDocumentContentProvider API](https://code.visualstudio.com/api/extension-guides/virtual-documents)
- [CommentController API](https://code.visualstudio.com/api/extension-guides/comments)

---

## **Summary**

- **Diff highlighting:** Use VS Code’s diff editor with virtual docs for red/green highlights.
- **File lists:** Use TreeDataProvider and your models to show changed files.
- **Comments:** Use CommentController API for in-diff commenting; handle storage yourself.
- **Arbitrary hunks:** Provide your own file contents via a content provider; VS Code will visualize them.

**The extension shows how to wire up these APIs for a PR, but you can use the same approach for any set of diffs you want to visualize.**
