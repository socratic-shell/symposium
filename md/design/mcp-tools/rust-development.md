# Rust Development Tools

The Rust development tools help agents work with Rust crates by providing access to source code, examples, and documentation.

## get_rust_crate_source

{RFD:rust-crate-sources-tool}

**Purpose**: Extract and optionally search Rust crate source code from crates.io

**Parameters**:
- `crate_name` (required): Name of the crate (e.g., "tokio")
- `version` (optional): Semver range (e.g., "1.0", "^1.2", "~1.2.3")
- `pattern` (optional): Regex pattern for searching within sources

**Behavior**:
- **Without pattern**: Extracts crate source and returns path information
- **With pattern**: Extracts crate source AND performs pattern search, returning matches

**Version Resolution**:
1. If `version` specified: Uses semver range to find latest matching version
2. If no `version`: Checks current project's lockfile for the crate version
3. If not in project: Uses latest version from crates.io

**Response Format**:

Without pattern (extraction only):
```json
{
  "crate_name": "tokio",
  "version": "1.35.0",
  "checkout_path": "/path/to/extracted/crate",
  "message": "Crate tokio v1.35.0 extracted to /path/to/extracted/crate"
}
```

With pattern (extraction + search):
```json
{
  "crate_name": "tokio",
  "version": "1.35.0", 
  "checkout_path": "/path/to/extracted/crate",
  "example_matches": [
    {
      "file_path": "examples/hello_world.rs",
      "line_number": 8,
      "context_start_line": 6,
      "context_end_line": 10,
      "context": "#[tokio::main]\nasync fn main() {\n    tokio::spawn(async {\n        println!(\"Hello from spawn!\");\n    });"
    }
  ],
  "other_matches": [
    {
      "file_path": "src/task/spawn.rs", 
      "line_number": 156,
      "context_start_line": 154,
      "context_end_line": 158,
      "context": "/// Spawns a new asynchronous task\n///\npub fn spawn<T>(future: T) -> JoinHandle<T::Output>\nwhere\n    T: Future + Send + 'static,"
    }
  ],
  "message": "Crate tokio v1.35.0 extracted to /path/to/extracted/crate"
}
```

**Key Features**:
- **Caching**: Extracted crates are cached to avoid redundant downloads
- **Project Integration**: Automatically detects versions from current Rust project
- **Example Priority**: Search results separate examples from other source files
- **Context Preservation**: Includes surrounding code lines for better understanding

**Common Usage Patterns**:
1. **Explore API**: `get_rust_crate_source(crate_name: "serde")` - Get crate structure
2. **Find Examples**: `get_rust_crate_source(crate_name: "tokio", pattern: "spawn")` - Search for usage patterns
3. **Version-Specific**: `get_rust_crate_source(crate_name: "clap", version: "^4.0", pattern: "derive")` - Target specific versions

This tool enables agents to provide accurate, example-driven assistance for Rust development by accessing real crate source code rather than relying on potentially outdated training data.
