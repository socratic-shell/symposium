# Installation

<div class="warning-banner">⚠️ Pre-alpha software: may eat your laundry</div>

## Supported

We aim to support as many tools as we can, but we currently have support only for a limited set. Currently supported tools:

* Editors
    * [VSCode](https://code.visualstudio.com/)
* Agentic tools
    * [Claude Code](https://github.com/anthropics/claude-code)
    * [Q CLI](https://github.com/aws/amazon-q-developer-cli)
    * you should be able to use it with any agent that does not support MCP, but it will require [manual configuration](#other-agents)
* Desktop platforms (not required)
    * Mac OS X

## Instructions

### Using the Symposium GUI app

If you are on a Mac, we recommend you use the Symposium GUI app. This app will allow you to have multiple [taskspaces](./learn-more/taskspaces.md) at once, letting you use many agents concurrently.

Steps to open the app:

* Clone the project from github
    * `git clone https://github.com/symposium-dev/symposium`
* To build and start the desktop app (OS X only):
    * `cargo setup --all --open`

### Using the VSCode plugin + the MCP server

If you don't want to use the GUI app, or you don't have a Mac, you can use the VSCode plugin and the MCP server independently:

* Clone the project from github
    * `git clone https://github.com/symposium-dev/symposium`
* To build and start the desktop app (OS X only):
    * `cargo setup --vscode --mcp`

### Using just the MCP server

You can also use *just* the MCP server. This will give access to some limited functionality such as the ablity to [fetch Rust crate sources](./learn-more/api-examples.md).

* Clone the project from github
    * `git clone https://github.com/symposium-dev/symposium`
* To build and start the desktop app (OS X only):
    * `cargo setup --mcp`

## Other agents

To use Symposium with another agent, you just need to add `symposium-mcp` as an MCP server. It will be installed in `~/.cargo/bin` if you use `cargo setup --mcp`.

But really the best would be to [contribute a patch to support your preferred agent!](../contribute.md)