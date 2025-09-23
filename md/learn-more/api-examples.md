# API Examples

Agents are quite good at using libraries that they know from training, but if you ask them to use a library they don't know well, they are prone to hallucination. One very simple way to help the agent figure out how to use a library is to give it access to the source; this is particularly effective for well-maintained libraries that include a lot of examples.

Symposium's MCP server includes a [tool for fetching and searching the sources of Rust crates](../design/mcp-tools/rust-development.md) that demonstrates this concept. This tool will point the agent at the sources for a library, either by finding them in your cargo cache or else by downloading them to a temporary directory. It will also search through the code for particular keywords (e.g., the name of an API the model is attempting to use).

## How to use it

Ask the model to "check the crate source" or "examine the crate source".

## How it's implemented

