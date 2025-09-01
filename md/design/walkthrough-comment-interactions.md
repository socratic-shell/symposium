# Walkthrough Comment Interactions

This document describes the design for interactive comment features in code walkthroughs, allowing users to reply to walkthrough comments and have those replies forwarded to the AI agent.

## When a walkthrough is presented

* Unambiguous comments (exactly one location) are added into the VSCode comment system immediately.
* Ambiguous comments: when the user clicks, they are presented with a dialogue to select how to resolve.
    * Once they select an option, the comment is rewritten to render as `file.rs:25` and the comment is placed.
    * A magnifying glass icon remains that, if clicked, will allow the user to reselect the comment placement.
        * When comment placement changes, the comment is moved to the new location in VSCode.

## Comment display

Comments display as if they were authored by "AI Agent" -- we should let AI agents customize their names later.

Comments have a "reply" button. When clicked, it inserts a [`<symposium-ref>`](./symposium-ref-system.md) that maps to a JSON blob like:

```json
{
    "in-reply-to-comment-at": {
        "file": "path/to/file.js",
        "start": {
            "line": 22,
        },
        "end": {
            "line": 44,
        },
        "comment": "... the text that the AI placed at this location ..."
    }
}
```

This is inserted into the AI chat and the user can type more.

## User-added comments

VSCode includes a `CommentProvider` that is meant to permit users to add comments on specific lines. Symposium registers as a comment provider but comments added by the user are written into the AI Agent chat instead.
