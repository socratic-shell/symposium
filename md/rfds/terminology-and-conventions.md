# RFD Terminology and Conventions

This document establishes standard terminology and conventions for use in Symposium RFDs to ensure consistency and clarity across all design documents.

## Terminology

### Agent
An **agent** refers to an LLM (Large Language Model) that is executing and interacting with the user within the Symposium environment. Agents have access to MCP tools and can perform actions on behalf of the user.

**Pronouns**: Always use "they/them" pronouns when referring to agents, not "it" or "he/she".

**Examples:**
- ✅ "When the agent needs to explore a crate, they will invoke the `get_rust_sources` tool"
- ✅ "The agent can use their access to the file system to read documentation"
- ❌ "When the agent needs to explore a crate, it will invoke the tool"
- ❌ "The agent can use his access to the file system"

### User
A **user** refers to the human developer interacting with Symposium and its agents.

### Tool
A **tool** refers to an MCP (Model Context Protocol) tool that agents can invoke to perform specific actions or retrieve information.

### Taskspace
A **taskspace** is an isolated working environment within Symposium where agents can work on specific tasks without interfering with other work.

## Writing Conventions

### Voice and Tone
- Use active voice when possible
- Write in present tense for current functionality, future tense for planned features
- Be specific and concrete rather than abstract
- Avoid unnecessary jargon or overly technical language

### Code Examples
- Use realistic examples that could actually occur in practice
- Include both the tool call and expected response when showing tool usage
- Use proper JSON formatting for MCP tool examples

### Formatting
- Use **bold** for emphasis on key terms when first introduced
- Use `code formatting` for tool names, function names, and technical terms
- Use bullet points for lists of features or requirements
- Use numbered lists for sequential steps or processes

## Revision History

- 2025-09-17: Initial terminology and conventions document
