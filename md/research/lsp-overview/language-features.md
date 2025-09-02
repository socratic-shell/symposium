# Language Features

Language Features provide the actual smarts in the language server protocol. They are usually executed on a [text document, position] tuple. The main language feature categories are: code comprehension features like Hover or Goto Definition. coding features like diagnostics, code complete or code actions.

## Navigation Features

### Go to Definition
```typescript
textDocument/definition: TextDocumentPositionParams → Location | Location[] | LocationLink[] | null
```

### Go to Declaration  
```typescript
textDocument/declaration: TextDocumentPositionParams → Location | Location[] | LocationLink[] | null
```

### Go to Type Definition
```typescript
textDocument/typeDefinition: TextDocumentPositionParams → Location | Location[] | LocationLink[] | null
```

### Go to Implementation
```typescript
textDocument/implementation: TextDocumentPositionParams → Location | Location[] | LocationLink[] | null
```

### Find References
```typescript
textDocument/references: ReferenceParams → Location[] | null

interface ReferenceParams extends TextDocumentPositionParams {
  context: { includeDeclaration: boolean; }
}
```

## Information Features

### Hover
```typescript
textDocument/hover: TextDocumentPositionParams → Hover | null

interface Hover {
  contents: MarkedString | MarkedString[] | MarkupContent;
  range?: Range;
}
```

### Signature Help
```typescript
textDocument/signatureHelp: SignatureHelpParams → SignatureHelp | null

interface SignatureHelp {
  signatures: SignatureInformation[];
  activeSignature?: uinteger;
  activeParameter?: uinteger;
}
```

### Document Symbols
```typescript
textDocument/documentSymbol: DocumentSymbolParams → DocumentSymbol[] | SymbolInformation[] | null
```

### Workspace Symbols
```typescript
workspace/symbol: WorkspaceSymbolParams → SymbolInformation[] | WorkspaceSymbol[] | null
```

## Code Intelligence Features

### Code Completion
```typescript
textDocument/completion: CompletionParams → CompletionItem[] | CompletionList | null

interface CompletionList {
  isIncomplete: boolean;
  items: CompletionItem[];
}

interface CompletionItem {
  label: string;
  kind?: CompletionItemKind;
  detail?: string;
  documentation?: string | MarkupContent;
  sortText?: string;
  filterText?: string;
  insertText?: string;
  textEdit?: TextEdit;
  additionalTextEdits?: TextEdit[];
}
```

**Completion Triggers:**
- User invoked (Ctrl+Space)
- Trigger characters (`.`, `->`, etc.)
- Incomplete completion re-trigger

### Code Actions
```typescript
textDocument/codeAction: CodeActionParams → (Command | CodeAction)[] | null

interface CodeAction {
  title: string;
  kind?: CodeActionKind;
  diagnostics?: Diagnostic[];
  isPreferred?: boolean;
  disabled?: { reason: string; };
  edit?: WorkspaceEdit;
  command?: Command;
}
```

**Code Action Kinds:**
- `quickfix` - Fix problems
- `refactor` - Refactoring operations
- `source` - Source code actions (organize imports, etc.)

### Code Lens
```typescript
textDocument/codeLens: CodeLensParams → CodeLens[] | null

interface CodeLens {
  range: Range;
  command?: Command;
  data?: any; // For resolve support
}
```

## Formatting Features

### Document Formatting
```typescript
textDocument/formatting: DocumentFormattingParams → TextEdit[] | null
```

### Range Formatting
```typescript
textDocument/rangeFormatting: DocumentRangeFormattingParams → TextEdit[] | null
```

### On-Type Formatting
```typescript
textDocument/onTypeFormatting: DocumentOnTypeFormattingParams → TextEdit[] | null
```

## Semantic Features

### Semantic Tokens
Since version 3.16.0. The request is sent from the client to the server to resolve semantic tokens for a given file. Semantic tokens are used to add additional color information to a file that depends on language specific symbol information.

```typescript
textDocument/semanticTokens/full: SemanticTokensParams → SemanticTokens | null
textDocument/semanticTokens/range: SemanticTokensRangeParams → SemanticTokens | null
textDocument/semanticTokens/full/delta: SemanticTokensDeltaParams → SemanticTokens | SemanticTokensDelta | null
```

**Token Encoding:**
- 5 integers per token: `[deltaLine, deltaStart, length, tokenType, tokenModifiers]`
- Relative positioning for efficiency
- Bit flags for modifiers

### Inlay Hints
```typescript
textDocument/inlayHint: InlayHintParams → InlayHint[] | null

interface InlayHint {
  position: Position;
  label: string | InlayHintLabelPart[];
  kind?: InlayHintKind; // Type | Parameter
  tooltip?: string | MarkupContent;
  paddingLeft?: boolean;
  paddingRight?: boolean;
}
```

## Diagnostics

### Push Model (Traditional)
```typescript
textDocument/publishDiagnostics: PublishDiagnosticsParams

interface PublishDiagnosticsParams {
  uri: DocumentUri;
  version?: integer;
  diagnostics: Diagnostic[];
}
```

### Pull Model (Since 3.17)
```typescript
textDocument/diagnostic: DocumentDiagnosticParams → DocumentDiagnosticReport
workspace/diagnostic: WorkspaceDiagnosticParams → WorkspaceDiagnosticReport
```

**Diagnostic Structure:**
```typescript
interface Diagnostic {
  range: Range;
  severity?: DiagnosticSeverity; // Error | Warning | Information | Hint
  code?: integer | string;
  source?: string; // e.g., "typescript"
  message: string;
  tags?: DiagnosticTag[]; // Unnecessary | Deprecated
  relatedInformation?: DiagnosticRelatedInformation[];
}
```
