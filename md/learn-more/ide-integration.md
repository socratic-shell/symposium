# IDE Integration for Context-Aware Discussions

The Symposium MCP server includes a [`ide_operations` tool](../design/mcp-tools/ide-integration.md) that lets your agent work directly with the IDE to perform common operations like "find references" or "find definitions". It also supports free-text searches through the workspace.

## How to use it

You shouldn't have to do anything, the IDE will simply make use of it when it sees fit.

## How it works

The MCP tool accepts programs written in the ["Dialect language"](../design/dialect-language.md), a very simple language that allows for complex expressions to be expressed. The intent is to eventually support a wide variety of operations and leverage smaller, dedicated models to translate plain English requests into those IDE operations. The MCP server then makes use of [IPC](../design/daemon.md) to communicate with the IDE and access primitive operations which are implemented using the IDE's native capabilities (e.g., in VSCode, by asking the relevant Language Server).