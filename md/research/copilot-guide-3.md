# Designing Commenting UI for Diff Views in a VS Code Extension

## **Goal**

Create a commenting experience in your VS Code extension similar to the GitHub PR review UI, with support for:
- Line and multi-line comments (e.g., shift-selecting a range)
- Comment threads that appear expanded by default
- Clickable comment icons in the gutter ("+" icon) that open the comment widget
- Additional features: "Make Code Suggestion" button, code suggestion formatting

---

## **How VS Code Native Commenting Works**

VS Code provides the [CommentController API](https://code.visualstudio.com/api/extension-guides/comments) for extensions to enable commenting in editors, including diff editors.

### **1. Multi-line Comments (Range Comments)**
- When creating a comment thread, you can specify the start and end of the range:
  ```typescript
  controller.createCommentThread(
    uri,
    new vscode.Range(startLine, 0, endLine, 0), // Multi-line range
    commentsArray
  );
  ```
- To support shift-selecting a range:
  - Your extension should register a `commentingRangeProvider` that returns all valid ranges (e.g., every line or every possible span).
  - VS Code will allow users to select a range (usually by shift-clicking, dragging, or keyboard), and will pass the selected range to your comment thread creation handler.

  **Reference Code:**
  - [CommentingRangeProvider API example](https://code.visualstudio.com/api/extension-guides/comments#enabling-commenting-ranges)
  - [GitHub PR extension: pulls in ranges for multi-line comments](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewCommentController.ts#L244-L258)

### **2. Expanded Comments by Default**
- By default, comment threads appear "collapsed" and expand on interaction.
- **To make them expanded by default:**  
  - Set the thread's `collapsibleState` property to `vscode.CommentThreadCollapsibleState.Expanded` when creating the thread:
    ```typescript
    thread.collapsibleState = vscode.CommentThreadCollapsibleState.Expanded;
    ```
  - When you create the thread (e.g., on editor open), set this property before adding it to the controller.

### **3. Clickable Comment Icon ("+" in Gutter)**
- The "+" icon is rendered by VS Code for lines/ranges returned by your `commentingRangeProvider`.
- **Making it clickable:**
  - Ensure your `commentingRangeProvider` returns valid ranges (single lines or multi-line spans).
  - Implement the handler for the comment creation event:
    - When the user clicks the "+" icon, VS Code calls your provider to create a new comment thread for the selected range.
    - Your code should create the thread and set its `canReply` property to `true`.
    - If clicking does nothing, check that:
      - Your `commentingRangeProvider` is registered and returns correct ranges
      - Your handler for comment thread creation is active and creates threads as expected

  **Reference Code:**
  - [CommentController and commentingRangeProvider registration example](https://code.visualstudio.com/api/extension-guides/comments#enabling-commenting-ranges)
  - [GitHub PR extension: comment thread creation logic](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewCommentController.ts#L244-L258)

### **4. "Make Code Suggestion" Button**
- The GitHub PR extension adds a button in the comment widget for code suggestions.
- You can implement a similar feature by:
  - Extending the comment input box with a command or context menu action that inserts a code suggestion block (e.g., `````diff\n...`````).
  - Listen for keyboard shortcuts (e.g., `ctrl+k m`) or provide a UI button.

  **Reference:**
  - [Changelog entry for "Suggest a Change"](https://github.com/microsoft/vscode-pull-request-github/blob/main/CHANGELOG.md#L918-L933)
  - [GIF demo](https://github.com/microsoft/vscode-pull-request-github/blob/main/documentation/changelog/0.58.0/suggest-a-change.gif)

---

## **Checklist for Implementation**

1. **Create a CommentController** at extension activation.
2. **Register a CommentingRangeProvider** that returns allowed commenting ranges (single or multi-line).
3. **Handle thread creation** so clicking "+" creates a thread and opens the comment widget.
4. **Set threads to expanded** by default using `collapsibleState`.
5. **Support multi-line comments** by passing a range to thread creation.
6. **Add code suggestion actions** via context menu or keyboard shortcut.
7. **Ensure canReply is true** for your threads to allow replies.

---

## **Troubleshooting Common Issues**

- **Clicking "+" does nothing:**  
  - Is the CommentController active for the editor URI?
  - Does the CommentingRangeProvider return valid ranges?
  - Is your thread creation handler implemented and creating threads with `canReply = true`?
- **Threads don't expand:**  
  - Are you setting `collapsibleState` to `Expanded` when creating threads?
- **Multi-line comments not supported:**  
  - Is your comment creation logic using the full range provided by VS Code?

---

## **References**

- [VS Code Comments API](https://code.visualstudio.com/api/extension-guides/comments)
- [CommentController example](https://github.com/microsoft/vscode-pull-request-github/blob/main/src/view/reviewCommentController.ts)
- [Changelog: Suggest a Change feature](https://github.com/microsoft/vscode-pull-request-github/blob/main/CHANGELOG.md#L918-L933)
- [Suggest a Change GIF](https://github.com/microsoft/vscode-pull-request-github/blob/main/documentation/changelog/0.58.0/suggest-a-change.gif)

---

## **Summary**

You can match (and extend) the GitHub PR commenting experience in your own extension using VS Code’s CommentController API:
- Multi-line support comes from passing ranges to thread creation.
- Expanded threads use `collapsibleState`.
- The "+" icon is native and becomes clickable when your provider and handlers are set up correctly.
- "Make Code Suggestion" is a custom action you can add to your comment UI.
