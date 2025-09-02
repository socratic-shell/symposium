# Local Pull Request Visualization Tool: Specification and Context

## **Goal**

Create a tool (ideally a VS Code extension) that visualizes the contents and changes in the current directory as if they were a GitHub Pull Request. This interface should:

- Show a list of "commits" representing:
  - `HEAD^` (the previous commit)
  - `HEAD` (the current commit)
  - **Staged changes** (as a synthetic commit)
  - **Unstaged changes** (as another synthetic commit)
- For each "commit," display the files changed.
- For each file, show a diff view that compares the relevant states (e.g., `HEAD^` vs `HEAD`, `HEAD` vs staged, staged vs unstaged).
- Allow users to leave comments on diffs, similar to the GitHub PR review experience.
- **Important:** Comments are handled by custom logic (not posted to GitHub); they are stored locally or sent to a custom backend.

---

## **Comparison to Existing Tools**

The [vscode-pull-request-github](https://github.com/microsoft/vscode-pull-request-github) extension provides a PR review UI for real GitHub PRs:
- Shows a tree of changed files
- Opens diffs for each file
- Allows comments on diffs
- Handles review actions (approve, request changes, etc.)

**However, this extension is tightly integrated with GitHub and actual PRs. My goal is to build a similar UI but for local changes and commits, not GitHub PRs.**

---

## **Desired Workflow**

1. **Scan Repo State:**
   - Identify current branch’s `HEAD` and `HEAD^` commits.
   - Detect staged and unstaged changes using git status/diff.

2. **Build a Synthetic "PR":**
   - Treat these four states (`HEAD^`, `HEAD`, staged, unstaged) as a sequence of commits.
   - List files changed at each step.

3. **Show UI:**
   - Display a tree/list of changed files for each commit/state.
   - Clicking a file opens a diff view (using VS Code’s diff editor API).
   - The diff views compare:
     - `HEAD^` vs `HEAD`
     - `HEAD` vs staged
     - staged vs unstaged

4. **Commenting:**
   - Users can leave comments on lines in the diff view.
   - Comments are managed by custom logic (use VS Code Comments API, but store locally/custom backend).

---

## **Technical Inspiration from Existing Extension**

- **Changed File Model:**  
  - See the extension’s `ReviewManager`, `PullRequestModel`, and related classes.
  - They model changed files, diffs, and comments per PR.

- **Diff View:**  
  - Uses VS Code’s `vscode.diff()` API to open editors showing diffs between commits (or working state).
  - For local implementation, diff views would compare git objects or workspace files.

- **Comments:**  
  - Uses CommentController API to display and manage comments on lines in diff view editors.
  - For local implementation, use similar API but store comments locally or in a custom backend.

- **Tree/List UI:**  
  - TreeDataProvider is used to show changed files in a PR.
  - For local implementation, present changed files per synthetic commit/state.

---

## **Implementation Notes**

- The extension need not interact with GitHub at all.
- Comment storage logic is entirely custom.
- The main challenge is mapping the current repo state to a sequence of commits and presenting those as a PR.

---

## **References**

- [VS Code Comment Controller API](https://code.visualstudio.com/api/extension-guides/comments)
- [VS Code Diff API](https://code.visualstudio.com/api/references/vscode-api#window)
- [vscode-pull-request-github](https://github.com/microsoft/vscode-pull-request-github)
  - See `src/view/reviewManager.ts`, `src/view/reviewCommentController.ts`, `src/view/pullRequestCommentController.ts` for relevant inspiration.

---

## **Example UI Flow**

1. User installs the extension, opens a repo.
2. Extension scans git history and status, builds a "local PR" model.
3. User opens the "Local PR" view:
   - Sees a list/tree of commits: `HEAD^`, `HEAD`, Staged, Unstaged.
   - Sees changed files for each.
4. Clicking a file opens a diff editor for the appropriate comparison.
5. User can leave comments in the diff view.
6. Comments are stored locally or in a backend, not on GitHub.

---

## **Open Questions**

- How should comments be persisted (local file? database? custom backend)?
- Should diffs for staged/unstaged be shown as synthetic commits, or as a single "workspace changes" diff?
- Should the extension support grouping or tagging comments?

---

## **Summary**

The goal is to build a “local PR visualization” tool in VS Code, inspired by the PR review UI, but operating purely on local repo state. The tool should show changed files and diffs for recent commits and workspace changes, and allow local commenting.
