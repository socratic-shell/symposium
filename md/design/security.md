# Security Considerations

*This section documents the security measures implemented in Dialectic's markdown rendering and IPC communication.*

## Webview Security Model

Dialectic renders markdown content in VSCode webviews, which operate in a sandboxed environment with strict security policies. Our security approach implements defense-in-depth with multiple layers of protection.

### Content Security Policy (CSP)

The webview includes proper CSP headers with nonce-based script execution:

```html
<meta http-equiv="Content-Security-Policy" 
      content="default-src 'none'; 
               script-src 'nonce-${nonce}'; 
               style-src ${webview.cspSource};">
```

**Key protections:**
- `default-src 'none'` - Blocks all resources by default
- `script-src 'nonce-${nonce}'` - Only allows scripts with the generated nonce
- Nonce generated using `crypto.randomBytes` for each render
- Prevents unauthorized script injection while allowing legitimate functionality

### HTML Sanitization

DOMPurify provides additional sanitization of the HTML output from markdown-it:

```javascript
const cleanHtml = DOMPurify.sanitize(renderedHtml, {
    ADD_ATTR: ['data-file-ref'],
    ALLOWED_TAGS: [...], 
    ALLOWED_ATTR: [...]
});
```

**Security benefits:**
- Runs in isolated JSDOM environment to prevent DOM manipulation attacks
- Configured to preserve necessary `data-file-ref` attributes for functionality
- Blocks potentially malicious content that could bypass CSP
- Defense-in-depth against XSS and other webview vulnerabilities

### Secure Link Handling

File references use data attributes instead of href-based manipulation:

```javascript
// ✅ Secure approach - data attributes
<a data-file-ref="src/auth.ts:23">src/auth.ts:23</a>

// ❌ Vulnerable approach - href manipulation  
<a href="javascript:openFile('src/auth.ts:23')">src/auth.ts:23</a>
```

**Advantages:**
- Prevents URL-based attacks and malformed protocol injection
- More controlled link processing through event delegation
- Prevents accidental navigation that could escape the webview context
- Clear separation between display and functionality

## IPC Communication Security

### Process Isolation

The MCP server and VSCode extension run as separate processes, providing natural security boundaries:

- **MCP Server**: Runs in AI assistant's process context with limited permissions
- **VSCode Extension**: Runs in VSCode's extension host with VSCode's security model
- **Communication**: Only through well-defined IPC protocol with structured messages

### Input Validation

All IPC messages undergo validation before processing:

```typescript
// Message structure validation
interface IPCMessage {
    id: string;           // UUID for request correlation
    type: string;         // Message type validation
    content: string;      // Markdown content (sanitized before rendering)
    mode: 'replace' | 'update' | 'append';  // Enum validation
}
```

**Validation layers:**
- JSON schema validation for message structure
- Content-type validation for markdown input
- Mode parameter validation against allowed values
- Error handling for malformed or oversized messages

## Threat Model

### What We're Protecting Against

**Primary threats:**
- Malicious markdown content injecting scripts or HTML
- Crafted file references attempting to access unauthorized locations
- IPC message injection or manipulation
- Webview escape attempts through malformed content

**Secondary considerations:**
- Accidental vulnerabilities from trusted AI assistant content
- Edge cases in markdown parsing that could be exploited
- Future expansion to untrusted content sources

### What We're Not Defending Against

**Out of scope:**
- Malicious AI assistants actively trying to attack the user (trust model assumes collaborative AI)
- VSCode extension host vulnerabilities (rely on VSCode's security model)
- Operating system level attacks (outside application boundary)
- Network-based attacks (all communication is local IPC)

## Security Best Practices

### For Contributors

When modifying security-sensitive code:

1. **Validate all inputs** at IPC and webview boundaries
2. **Use parameterized queries** for any dynamic content generation
3. **Test with malicious inputs** including script tags, unusual protocols, oversized content
4. **Follow principle of least privilege** - only enable minimum required capabilities
5. **Update CSP headers** when adding new webview functionality

### For AI Assistants

When generating review content:

1. **Use standard markdown syntax** - avoid HTML tags unless necessary
2. **Validate file paths** before including in reviews
3. **Keep content reasonable size** to avoid resource exhaustion
4. **Use standard file:line reference format** for navigation links

## Security Updates

This security model was implemented as part of the markdown rendering architectural refactor (July 2024). Key improvements over the previous approach:

- Replaced fragile VSCode internal APIs with industry-standard markdown-it
- Added comprehensive CSP and DOMPurify sanitization
- Implemented secure data-attribute-based link handling
- Established clear security boundaries between components

Future security enhancements should maintain these defense-in-depth principles while enabling new functionality.
