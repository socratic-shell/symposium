# Code walkthroughs

Code walkthroughs are triggered in circumstances like:

1. the AI agent completes a set of related changes or encounters an obstacle where it requires the users help
2. the user asks to review the agent's work
3. the user asks the agent to explain a given codepath or to walk through some portion of the code.

In these cases, the agent triggers the `present_walkthrough` tool. Walkthroughs are usually (but not always) related to code; they can also be useful for documents or other files, however.

## Example of a walkthrough

We begin with examples before defining the full structure. A *walkthrough* is defined by a JSON object with a standard set of actions. All fields are optional.

```json
{
    // The introduction section covers the .
    "introduction": [
        "These entries are markdown paragraphs.",
        "Each paragraph can be a separate entry."
    ],

    // The "highlights" section is used to highlight areas
    // that the agent wishes to call attention to. These are
    // either key points in the walkthrough.
    "highlights": [
        {
            // A "comment" leaves a comment on some location of the document
            "comment": {
                // Comments can optionally have an icon using vscode's standard "codicons"
                "icon": "$(question)", // e.g.

                // Where to put the comment. In this case, the comment is located based on search files.
                "location": {"search": {"file": "src/foo.rs", "regex": "fn foo\\b" }},

                // The question being asked.
                "content": [
                    "I was not sure what the name of this function should be. Please double check it!"
                ],
            },
        }
    ],

    // The "changes" section is used to document the full set of changes.
    "changes": [
        {
            // A "diff" presents changes from a range of git commits.
            // It can also include 
            "gitdiff": {
                // The range can be a range of commits (e.g., `HEAD~3..HEAD~1`)
                // of a single commit. 
                "commit_range": "HEAD^..",

                // If the range includes HEAD, then by default we will include
                // unstaged and staged changes. The excluded parameter can
                // be used to exclude those.
                "exclude": {
                    "unstaged": false,
                    "staged": false
                }
            }
        }
    ]

    // The actions section is used to give the user choices on how
    // to proceed. Actions can technically be embedded anywhere.
    //
    // The following is the default actions if none are otherwise given.
    "actions": [
        {
            "action": {
                // Description
                "description": "If you are satisfied with these changes, checkpoint them to update tracking documents.",

                // Text on the button.
                "button": "Checkpoint",

                // Text sent to the agent
                "tell_agent": "Checkpoint"
            },

             "action": {
                // Description
                "description": "Request the agent to make more changes.",

                // Text on the button.
                "button": "Request changes",

                // Text sent to the agent
                "tell_agent": "I'd like to make more changes. Please discuss with them with me."
            }
        },
    ]
}
```

## Interaction

When the MCP tool is triggered, the Symposium pane renders the given walkthrough.
It also renders a "clear" button to clear the current code review.

### Rendering the walkthrough

Markdown and mermaid are rendered in the "obvious" way.

Users can click on comments to be taken to the source code line.
Comments are also added using vscode's comment API.

Comments use regex-based positioning to locate the relevant code. When a regex matches multiple locations, the system presents a QuickPick to let the user navigate and select the intended location. If a user clicks on an ambiguous comment again, they get a chance to reselect in case they made a mistake. If they choose differently, the comment will be moved in the comment controller to the new location.

When a walkthrough is cleared or a new walkthrough is loaded, all existing comments are removed from the comment controller.

"gitdiff" elements are rendered as a tree:

* "all commits" (only if there's more than one)
    * "file1.rs" (+/-) -- see changes made to file1.rs across all commits
    * "file2.rs" (+/-) -- see changes made to file2.rs across all commits
* "sha1"
    * "file1.rs" (+/-) -- see changes made to file1.rs in in commit sha1
* "sha2"
    * "file2.rs" (+/-) -- see changes made to file2.rs in in commit sha2

Files that have comments added elsewhere also have a "discussion" icon.
Clicking on the icon.

### Actions

When the user clicks the action button, the given text is sent to the LLM using the [Discuss in Symposium](./discuss-in-symposium.md) mechanism and the terminal is selected. To avoid triggering action, newlines are stripped from the string and replaced with spaces. Users can press enter themselves.

### Responding to comments

Each comment has a button on it that says "Reply".

Clicking that button sends text to the LLM using the [Discuss in Symposium](./discuss-in-symposium.md) mechanism:

```
<context>In reply to comment on file.rs?regex (line XXX).</context>
```

and selects the terminal so that user can continue typing.

## Walkthrough format

Walkthroughs are a JSON value that contains four (optional) parts:

```
Walkthrough = {
    ("introduction": List)?,
    ("highlights": List)?,
    ("changes": List)?,
    ("actions": List)?
}
```

Each `List` is in fact a [Dialect program](./dialect-language.md) that can be executed to yield up a list of content items for rendering.
Dialect allows them to embed references to [IDE operations](./ide-capabilities.md) like finding references or definitions of symbols.
The grammar is as follows:

```
// Lists are in fact Dialect programs that can be directly executed
// to yield the final result.
List = "Markdown text"     // shorthand for ["Markdown text"]
     | [ ListElement* ]    // series of rows

ListElement = "Markdown text"               // render as markdown
            | {                             // render an action
                "action": {
                  "content": List,  // show user this markdown
                  "button": "button text / icon",  // render a button with this text
                  ("tell_agent": "string")?        // message to send to agent when button is clicked
                }
            }
            | {                             // render a set of diffs from git
                "gitdiff": {
                    "range": "...",
                    ("exclude": {
                        ("unstaged": boolean)?,
                        ("staged": boolean)?
                    })?
                }
            }
            | {
                "comment": {
                    "location": Location,
                    ("icon": "icon")?,
                    content: List
                }
            }

// Locations are dialect functions that yield lists of positions or ranges.
// If exactly one match is found, then the comment is placed on that line.
// If multiple matches are found, users are prompted to disambiguate when they click on the comment.
Location    = { "search": { "path": "path", "regex": "regex to search for" } // if `path` is a directory, search all files within
            | { "range": { "path": "path", line: number, (column:number)? } } // a range of text
            | { "findReferences": "MyClass" }                                // comment on reference(s) to MyClass
            | { "findDefinitions": "MyClass" }                               // comment on definition(s) of MyClass
```

The final resulting datastructures in a List are as follows:

* Each list element is a Dialect value that evaluates to itself (e.g., `comment`).
* Locations are a list of *locations*, which is a JSON struct like `{file:"filename", line, column}`. They may have additional fields.

## Future Extensions

### Mermaid diagrams

Support for mermaid diagrams in walkthrough content:

```json
{
    "mermaid": "graph TD; A-->B; B-->C;"
}
```

This would render interactive diagrams within walkthroughs for visualizing architecture, flows, or relationships.
