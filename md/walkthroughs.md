# Code walkthroughs and Ask Symposium

This document defines the expected user workflows for walkthrough features in Dialectic.

## Comment Interaction Workflow

### Expected User Flow: Single Location Comment
1. **AI creates walkthrough** with `<comment location="..." icon="...">text</comment>`
2. **User sees clickable comment** in walkthrough panel showing exact location (e.g., "src/main.rs:42 🔍")
3. **User clicks location text** → VSCode opens file and jumps to line 42
4. **Comment thread appears** in the editor with the explanation text
5. **User can reply** to the comment thread to continue the conversation with the AI
6. **User can click magnifying glass** → shows location picker to reposition comment (same as ambiguous flow)

### Expected User Flow: Multiple Location Comment (Ambiguous)
1. **AI creates walkthrough** with comment that matches multiple locations
2. **User sees clickable comment** showing "(X possible locations) 🔍" 
3. **User clicks comment** → VSCode shows QuickPick with all matching locations
4. **User navigates through options** → VSCode shows source code preview for each location (like "Go to References")
5. **User selects specific location** from the picker
6. **Comment gets placed** at chosen location in the editor
7. **🚨 CRITICAL: Walkthrough UI updates** to show the chosen location (e.g., "src/main.rs:42 🔍") with magnifying glass icon
8. **Subsequent clicks** on the location text go directly to the chosen location
9. **User can click magnifying glass** → clears current comment placement and shows location picker again
10. **User selects new location** → comment moves from old location to new location in editor
11. **Walkthrough UI updates** to show the new chosen location

### Expected User Flow: No Location Found
1. **AI creates walkthrough** with invalid/unresolved location expression
2. **User sees comment** showing "no location" or error message
3. **Comment is not clickable** (or shows error when clicked)
4. **User can still read** the comment text for context

## Action Button Workflow

### Expected User Flow: Action Buttons
1. **AI creates walkthrough** with `<action button="text">message</action>`
2. **User sees styled button** with the specified text
3. **User clicks button** → message is sent to terminal where AI assistant is running
4. **AI receives message** and can respond or take further action
5. **Button remains functional** for repeated use

## Mermaid Diagram Workflow

### Expected User Flow: Mermaid Diagrams
1. **AI creates walkthrough** with `<mermaid>diagram code</mermaid>`
2. **User sees rendered diagram** inline with walkthrough content
3. **Diagram uses VSCode theme colors** for consistency
4. **Diagram is static** (no interaction beyond viewing)

## GitDiff Display Workflow

### Expected User Flow: Git Diffs
1. **AI creates walkthrough** with `<gitdiff range="HEAD~1..HEAD" />`
2. **User sees diff tree** showing changed files with +/- statistics
3. **User can click file names** to open diffs in VSCode
4. **Diff content displays** in VSCode's native diff viewer

## Current Known Issues

### 🚨 Comment Location Update Issue
**Problem**: After selecting a location from an ambiguous comment picker, the walkthrough UI still shows "(X possible locations)" instead of updating to show the chosen location.

**Expected Fix**: The walkthrough comment should update its display to show the specific chosen location.

**Technical Note**: This requires the extension to send an update message back to the walkthrough webview when a location is chosen.

### 🚨 Comment Persistence Issue  
**Problem**: User replies to comment threads persist in the UI after sending to terminal.

**Expected Behavior**: Comment thread should either clear the reply or indicate it was sent to the AI.

### 🚨 Symposium-Ref Resolution Issue
**Problem**: Symposium references (`<symposium-ref id="..."/>`) fail to resolve.

**Expected Behavior**: References should expand to provide context to the AI assistant.
