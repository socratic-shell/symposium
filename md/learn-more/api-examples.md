# API Examples

*Helping LLMs find Rust examples and crate sources*

## What are API Examples?

API Examples is a Symposium feature that helps AI assistants discover and understand Rust crate APIs by providing access to real-world usage examples and source code. This bridges the gap between documentation and practical implementation.

## The Problem

When working with Rust crates, AI assistants often struggle with:
- **Outdated training data** - Models may not know about recent crate versions
- **Limited examples** - Documentation often lacks comprehensive usage patterns
- **API evolution** - Breaking changes between versions aren't always clear
- **Best practices** - Knowing the idiomatic way to use a crate's API

## How It Works

### Crate Source Access
- Direct access to crate source code from crates.io
- Version-specific source browsing
- Pattern matching across implementation files
- Understanding of internal structure and design patterns

### Example Discovery
- Real usage examples from the crate's own tests
- Documentation examples with full context
- Common patterns and idioms used by the crate authors
- Error handling and edge case examples

### Contextual Understanding
- API relationships and dependencies
- Type system integration
- Trait implementations and generic usage
- Macro expansion and procedural macro patterns

## Benefits for Developers

### Faster Learning
- Understand new crates through concrete examples
- See how APIs are intended to be used
- Learn idiomatic Rust patterns from crate authors
- Discover advanced features through source exploration

### Better Code Quality
- Follow established patterns and conventions
- Understand error handling approaches
- Learn performance considerations from source
- Avoid common pitfalls and anti-patterns

### Reduced Context Switching
- Get examples directly in your development environment
- No need to search through documentation websites
- Immediate access to source code for deeper understanding
- Integrated explanations alongside code examples

## Use Cases

### API Exploration
```rust
// AI can show you how tokio::spawn is actually used
// by finding examples in the tokio source and tests
use tokio::task;

let handle = task::spawn(async {
    // Real examples from tokio's own codebase
    println!("Running in background task");
});
```

### Pattern Discovery
- Understanding builder patterns in specific crates
- Learning async/await usage patterns
- Discovering trait implementation strategies
- Finding macro usage examples

### Troubleshooting
- Seeing how crate authors handle edge cases
- Understanding error types and handling strategies
- Finding workarounds for known limitations
- Learning debugging techniques from tests

## Integration with Symposium

### Taskspace Integration
- API examples work seamlessly within taskspaces
- Source code exploration doesn't affect your main project
- Easy experimentation with different approaches
- Safe learning environment for trying new patterns

### Interactive Walkthroughs
- Generate walkthroughs showing API usage progression
- Explain complex patterns with step-by-step breakdowns
- Visual diagrams of type relationships and data flow
- Interactive exploration of crate internals

### Collaborative Learning
- Ask questions about specific API design decisions
- Understand the reasoning behind implementation choices
- Learn from the crate authors' problem-solving approaches
- Build deeper understanding through guided exploration

## Getting Started

1. **Ask about a crate** - "How do I use serde for JSON serialization?"
2. **Request examples** - "Show me real examples of tokio::select usage"
3. **Explore source** - "How does clap implement its derive macros?"
4. **Learn patterns** - "What are common error handling patterns in this crate?"

The AI assistant will use API Examples to provide concrete, up-to-date information with real source code references and practical usage patterns.

## Future Enhancements

- Support for more languages beyond Rust
- Integration with package managers (npm, PyPI, etc.)
- Community example contributions
- Performance benchmarking integration
- Automated example validation and updates

API Examples transforms how developers learn and work with external libraries by providing direct access to authoritative sources and real-world usage patterns.
