# Installation

This project is still under **heavy development**. Be aware that it may "eat your laundry".

## Supported

We aim to support as many tools as we can, but we currently have support only for a limited set. Currently supported tools:

* Editors
    * [VSCode](https://code.visualstudio.com/)
* Agentic tools
    * [Claude Code](https://github.com/anthropics/claude-code)
    * [Q CLI](https://github.com/aws/amazon-q-developer-cli)
* Desktop platforms (not required)
    * Mac OS X

## Instructions

* Install and configure
    * A supported agentic tool
    * A supported editor
* Clone the project from github
    * `git clone https://github.com/symposium-dev/symposium`
* To build and start the desktop app (OS X only):
    * `cargo setup --all --open`
* To install the IDE + MCP tool:
    * Run `cargo setup --vscode --mcp`

