# Interactive Walkthroughs

Interactive walkthroughs let agents present code explanations with visual diagrams, comments, and interactive elements directly in your IDE.

## Where it's useful

Walkthroughs are great for

* reviewing code that an agent just finished writing;
* diving into a new codebase;
* debugging a problem within your codebase.

## How to use it

* Ask the agent to "present me a walkthrough"; the side panel should open.
* For code comments:
    * Click on comments to locate them in the source.
    * Click the "Reply" button to embed a [`<symposium-ref/>`](./symposium-ref.md) that will tell the agent what comment you are responding to, and then talk naturally!
* You can also select text in any editor and use the "Discuss in Symposium" action to get a [`<symposium-ref/>`](./symposium-ref.md) referring to that text.

## How it works

The MCP server offers a [`present_walkthrough` tool](../design/mcp-tools/walkthroughs.md). Agents invoke this tool with markdown that includes [special blocks](../design/walkthrough-format.md) for coments and the like. The MCP server uses [IPC](../design/ipc_message_type_reference.md) to connect to the IDE extension. [Read more in the design and implementation section.](../design/walkthroughs.md).