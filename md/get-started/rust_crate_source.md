# Get Rust Crate Sources

When agents encounter libraries not in their training data, fetching the source code is often the fastest way to understand the API. The Symposium MCP server includes a tool that will download the crate source and make it available to your agent; it tries to be smart by matching the version used in your project and pointing the agent at the cached copy from `~/.cargo` when available. If no cached copy is available though it will create a temporary directory.

To try this out, come up with a task that requires understanding some non-trivial API that is also not very common. (For example, perhaps creating a mdbook annotation processor.) Most agents will hallucinate methods they feel "ought" to be part of the API, result in a lot of churn, even if they do eventually succeed. But if you remind them to "fetch the crate source", they ought to do much better!

## Identify examples from Rust crate conventions

The tool attempts to leverage the convention of putting API examples in `examples` or rustdoc comments. Agents can include a search term when fetching the crate source and the tool will highlight matches that occur in examples in particular.

## Additional capabilities for code generation

Besides fetching crate sources, Symposium's MCP server includes (or plans to include...) other tools aimed helping agents generate better code:

* [IDE operations](../ref/ide-integration.md) let the agent find references or fetch type information.
