# Test Mermaid Code Block Syntax

This is a test walkthrough to verify the new mermaid code block syntax works correctly.

## Architecture Overview

Here's how the system works:

```mermaid
flowchart TD
    A[User Input] --> B{Parser}
    B -->|Code Block| C[Process Mermaid]
    B -->|XML| D[Process XML]
    C --> E[Generate HTML]
    D --> E
    E --> F[Render in Webview]
```

## Another Diagram

This one has blank lines to test the original issue:

```mermaid
sequenceDiagram
    participant U as User
    participant P as Parser
    participant R as Renderer

    U->>P: Send markdown
    
    P->>P: Parse code blocks
    
    P->>R: Generate HTML
    R->>U: Display walkthrough
```

The new syntax should handle both diagrams correctly without the blank line parsing issues.
