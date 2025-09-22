# Dialect Language

The Dialect language is a superset of JSON with function call syntax for expressing and composing IDE operations. Any valid JSON is also valid Dialect.

## Design Goals

Dialect is designed to be **LLM-friendly** - the syntax should feel natural and familiar to language models, matching the kind of pseudo-code they would generate intuitively:

- **Function call syntax**: `findDefinitions("MyClass")` reads like natural pseudo-code
- **JSON superset**: We accept JSON augmented with function calls but we are also tolerant of trailing commas, unquoted field names

The goal is to minimize the gap between "what an LLM wants to express" and "valid Dialect syntax", making code generation more reliable and the language more intuitive.

## Quick Start

**Find where a symbol is defined:**
```
findDefinitions("MyFunction")
```

**Find all references to a symbol:**
```
findReferences("MyClass")
```

**Get information about a symbol:**
```
getSymbolInfo("methodName")
```

**Composition - find references to all definitions:**
```
findReferences(findDefinitions("MyFunction"))
```

## Grammar

```
Program = Expr

Expr = FunctionCall
     | JsonObject  
     | JsonArray
     | JsonAtomic

FunctionCall = Identifier "(" ArgumentList? ")"

ArgumentList = Expr ("," Expr)* ","?

JsonObject = "{" (JsonProperty ("," JsonProperty)* ","?)? "}"
JsonProperty = (String | Identifier) ":" Expr

JsonArray = "[" (Expr ("," Expr)* ","?)? "]"

JsonAtomic = Number | String | Boolean | "null" | "undefined"

Identifier = [a-zA-Z_][a-zA-Z0-9_]*
String = "\"" ... "\""  // JSON string literal
Number = ...            // JSON number literal  
Boolean = "true" | "false"
```

## Function Signatures

Functions are called with positional arguments in a defined order:

### Core IDE Operations
- `findDefinitions(symbol: string)` - Find where a symbol is defined
- `findReferences(symbol: string)` - Find all references to a symbol  
- `getSymbolInfo(symbol: string)` - Get detailed symbol information

### Search Operations  
- `searchFiles(pattern: string, path?: string)` - Search for text patterns
- `findFiles(namePattern: string, path?: string)` - Find files by name

## Dynamic Semantics

A Dialect expression `E` evaluates to a JSON value:

### Function Calls
* If `E = Identifier(Expr...)`, then:
    * Evaluate each `Expr` to values `V...`
    * Look up the function `Identifier` 
    * Call the function with positional arguments `V...`
    * Return the function's result

### JSON Values
* If `E = [ Expr... ]`, evaluate each `Expr` to `V...` and return `[ V... ]`
* If `E = { Property... }`, evaluate each property value and return the object
* If `E = number | string | boolean | null | undefined`, evaluate to itself

## Implementation

The parser is implemented in `dialect/parser.rs`. The interpreter in `dialect.rs` handles function dispatch.

### Defining Functions

Functions implement the `DialectFunction` trait with parameter order specification:

```rust
{{#include ../../symposium/mcp-server/src/dialect.rs:dialect_function_trait}}
```

Functions that represent values can implement `DialectValue` instead:

```rust
{{#include ../../symposium/mcp-server/src/dialect.rs:dialect_value_trait}}
```

## Error Handling

The parser provides detailed error messages with source location indicators:
```
error: Expected ')'
  |
1 | findDefinitions("MyClass"
  |                          ^
  |
```

