# RFC: Scripting Language

*Generic JSON mini-language for composable operations*

## Overview

The IDE capability tool uses an internal JSON-based scripting language to represent operations precisely. **The language itself is completely agnostic** - it knows nothing about IDEs, symbols, or code. All domain-specific functionality is provided through extension points.

## Design Principles

### Made to be authored by LLMs
Programs in this language will be written by LLMs and we are designing with them in mind. We choose JSON because it is a familiar syntax so they can leverage their training better. Functions coerce arguments in obvious ways since LLMs will not necessarily think to do so.

### Empower the LLM to resolve ambiguity
When there are multiple ambiguous meanings, we should highlight the possible meanings  to the client LLM using natural language. It can decide how to proceed rather than us attempt to guess. 

### Simple, unambiguous, and extensible core
We are designing a core language that has a clean conceptual mapping (it is essentially a lambda calculus, in fact) and which can be readily extended to support all kinds of IDE and language specific functionality.

## Core Language Concepts

### JSON Objects as Function Calls

The fundamental concept is that **JSON objects represent function calls**:
- **Field name** = function name
- **Field value** = arguments object (with named parameters)

```json
{"functionName": {"arg1": "value1", "arg2": "value2"}}
```

### Function Composition and Automatic Type Coercion

Functions can be nested as arguments to other functions:

```json
{
  "outerFunction": {
    "parameter": {"innerFunction": {"arg": "value"}}
  }
}
```

**Automatic Type Coercion**: When a function receives an argument of a different type than expected, the runtime automatically attempts conversion using registered type converters. For example:

```json
{
  "findReferences": {
    "symbol": {"getSelection": {}}
  }
}
```

If `findReferences` expects a `symbol` but `getSelection` returns a `selection`, the runtime automatically attempts `selection → symbol` conversion. If the conversion succeeds, the function proceeds normally. If conversion fails or is ambiguous, the function returns a structured error with suggestions.

### Function Execution Outcomes

Functions in the language have three possible execution outcomes:

**Success**: Function completes and returns a value (which may be a constructor call for custom types)

**Unrecoverable Failure**: Function encounters an error that cannot be resolved by the user (e.g., network timeout, file system error). These propagate as standard errors.

**Ambiguous Failure**: Function finds multiple valid results and needs user guidance to proceed. These failures include structured data that the runtime converts into refinement suggestions like:

```
"The symbol `validateToken` is defined in multiple places. Which did you mean?

If you meant the `validateToken` from `auth.ts:42`, use `{"findSymbol": {"name": "validateToken", "file": "auth.ts", "line": 42}}`.

If you meant the `validateToken` from `utils.ts:15`, use `{"findSymbol": {"name": "validateToken", "file": "utils.ts", "line": 15}}`.
```

This three-outcome model enables the self-teaching behavior where ambiguous failures guide users toward successful usage rather than simply failing.

### Built-in Types

The language runtime provides only minimal built-in types:

**JSON-Native Types:**
- `string`: Text values
- `number`: Numeric values  
- `boolean`: True/false values
- `array`: Lists of values
- `object`: Key-value maps

**Built-in Conversions:**
- String ↔ Number (when valid)
- Boolean ↔ Number (0/1)
- Array ↔ String (join/split operations)

### Extension Points

All domain-specific functionality is provided through three extension mechanisms:

## Signature Definition

Function and type signatures use a common parameter schema system that supports optional parameters:

```typescript
interface ParameterSchema {
  [key: string]: string | dialectic.Optional<string>;
}

// dialectic.Optional wraps a type to indicate it's optional
namespace dialectic {
  export function Optional<T>(type: T): OptionalType<T>;
}
```

**Examples:**
```typescript
// Required parameters only
{name: "string", file: "string", line: "number"}

// Mixed required and optional parameters  
{name: "string", file: dialectic.Optional("string"), line: dialectic.Optional("number")}
```

When optional parameters are omitted, functions should attempt to infer reasonable values or return ambiguous failures with suggestions for disambiguation.

#### Base Callable Interface

All callable entities (functions and type constructors) share a common base class:

```typescript
abstract class Callable {
  name: string;           // Function/constructor name
  description: string;    // Natural language description for LLMs
  implementation: Function; // Actual implementation
  parameters: ParameterSchema; // Parameter types (required and optional)
  
  constructor(name: string, description: string, implementation: Function, parameters: ParameterSchema) {
    this.name = name;
    this.description = description;
    this.implementation = implementation;
    this.parameters = parameters;
  }
}
```

#### 1. Type Descriptions

Type constructors are defined using classes that extend the base Callable:

```typescript
class TypeDescription extends Callable {
  jsClass: Function;      // JavaScript class for instanceof checks
  
  // Type constructors return instances of themselves
  get returns(): string {
    return this.name;
  }
  
  constructor(name: string, description: string, implementation: Function, 
              parameters: ParameterSchema, jsClass: Function) {
    super(name, description, implementation, parameters);
    this.jsClass = jsClass;
  }
}
```

**Example:**
```typescript
new TypeDescription(
  "symbol",
  "Represents a code symbol like a function, variable, or type definition",
  createSymbol,
  {
    name: "string", 
    file: dialectic.Optional("string"), 
    line: dialectic.Optional("number")
  },
  Symbol
)
```

#### 2. Function Descriptions

Functions are defined using classes that extend Callable and add return type information:

```typescript
class FunctionDescription extends Callable {
  returns: string;        // Return type
  
  constructor(name: string, description: string, implementation: Function,
              parameters: ParameterSchema, returns: string) {
    super(name, description, implementation, parameters);
    this.returns = returns;
  }
}
```

**Example:**
```typescript
new FunctionDescription(
  "findSymbol",
  "Find a code symbol by name, optionally narrowed by file and line",
  findSymbolImplementation,
  {
    name: "string",
    file: dialectic.Optional("string"),
    line: dialectic.Optional("number")
  },
  "symbol"
)
```

#### 3. Type Conversions

Type conversions are defined using classes for consistency and better type safety:

```typescript
class TypeConversion {
  fromType: string;       // Source type name
  toType: string;         // Target type name
  description: string;    // Natural language description of the conversion
  converter: Function;    // Conversion implementation
  
  constructor(fromType: string, toType: string, description: string, converter: Function) {
    this.fromType = fromType;
    this.toType = toType;
    this.description = description;
    this.converter = converter;
  }
}
```

**Example:**
```typescript
new TypeConversion(
  "selection",
  "symbol",
  "Extract the symbol reference from a text selection",
  (selection) => extractSymbolFromSelection(selection)
)
```

## Constructor Functions and Self-Evaluation

Values created by constructor functions serialize as executable JSON programs:

```json
// A constructor might return:
{
  "customType": {
    "property1": "value1",
    "property2": 42,
    "nestedValue": {"otherType": {"data": "example"}}
  }
}
```

This return value is itself an executable program that would recreate the same value if executed.

## Example Usage (IDE Domain)

*Note: These examples show how the generic language might be used for IDE operations, but the language itself knows nothing about IDEs.*

### Simple Operations
```json
{"getCurrentSelection": {}}
{"findByName": {"name": "validateToken"}}
```

### Composed Operations
```json
{
  "findReferences": {
    "target": {"getCurrentSelection": {}}
  }
}
```
*Automatic conversion from selection to symbol*

## Implementation Architecture

### Language Runtime
- **Parser**: Validates JSON structure and function call format
- **Executor**: Recursively evaluates nested function calls
- **Type System**: Manages automatic conversions and type checking
- **Error Handler**: Provides structured error messages with suggestions

### Extension Registry
- **Type Registry**: Manages custom value types and their constructors
- **Function Registry**: Maps function names to implementations
- **Conversion Registry**: Handles automatic and explicit type conversions

### Execution Flow
1. Parse JSON program into function call tree
2. Resolve function names against registry
3. Attempt automatic type conversions for arguments
4. Execute functions recursively (inner functions first)
5. Return results in executable JSON format

## Open Design Questions

The core language design is fairly clear, but several questions need resolution:

### 1. [Validation Boundaries](./scripting-language/validation-boundaries.md)

Where should type checking and argument validation happen?
- In the language runtime (engine validates before calling functions)?
- In the function implementations (functions validate their own arguments)?
- Some hybrid approach?

### 2. [Ambiguity Resolution](./scripting-language/ambiguity-resolution.md)

How should functions implement ambiguity handling and error propagation?
- What's the mechanism for functions to signal ambiguous results (return error objects vs throw exceptions)?
- How do errors propagate through function composition chains?
- What's the format for suggestion data that gets converted to user-facing refinement options?
- How does the runtime convert function-level errors into natural language suggestions?

### 3. Future Implementation Details

Additional areas that need specification:
- **Error format specification**: Actual data structures for the three outcome types
- **Extension registry mechanics**: How extensions are loaded and registered at runtime
- **Async operation handling**: How the runtime manages async VSCode operations transparently
- **Memory management**: Cleanup strategies for opaque values when programs complete

## Benefits of This Design

**Domain Agnostic**: The language can be used for any domain, not just IDE operations
**Composable**: Functions naturally combine to create complex operations
**Extensible**: Easy to add new types, functions, and conversions
**Self-documenting**: The JSON structure shows exactly what operations are being performed
**Type-safe**: The runtime can validate that functions receive appropriate value types
**Debuggable**: Programs are human-readable and can be inspected/modified
**Lambda calculus-like**: Programs and data have the same representation

## Next Steps

1. Resolve the validation boundaries question
2. Design the ambiguity resolution mechanism  
3. Implement a basic runtime with extension point interfaces
4. Create example extensions for IDE operations
5. Test composition and error handling with real examples
