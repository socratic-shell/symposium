# CGS API Security Restrictions Research Report

*Findings from testing Core Graphics Services APIs for cross-application window management*

## Executive Summary

Our empirical testing of CGS APIs revealed fundamental security restrictions that prevent applications from controlling other applications' windows, even with proper accessibility permissions. The key finding is that **applications can only manipulate their own windows** using CGS APIs, not windows belonging to other applications.

## Test Results

### Chrome Window Test Results
Testing CGS APIs on a Google Chrome window (ID: 122344):

**✅ What Worked:**
- `CGSSetWindowLevel` - All level changes reported success 
- `CGSSetWindowAlpha` - All transparency changes reported success

**❌ What Failed:**
- `CGSOrderWindow` - All ordering operations failed with error 1000 (`kCGErrorFailure`)
- No visual changes observed despite "success" status codes

### Terminal Window Test Results  
Testing CGS APIs on a Terminal window (ID: 122403):

**✅ What Worked:**
- `CGSSetWindowLevel` - All level changes reported success
- `CGSSetWindowAlpha` - All transparency changes reported success (including 0% alpha)

**❌ What Failed:**
- `CGSOrderWindow` - All ordering operations failed with error 1000
- **No visual changes observed** despite successful alpha=0 API call

### Critical Discovery: "Silent Failure" Mode

The most significant finding is that CGS APIs exhibit **"silent failure" behavior** when manipulating other applications' windows:

1. **API calls return success codes** (`noErr`) 
2. **No visual changes actually occur**
3. **Only ordering operations explicitly fail** with error 1000
4. **Level and alpha changes are silently ignored**

## Root Cause: macOS Security Model

### "Universal Owner" Privilege Restriction

From our research in `how-mac-os-applications-respond-to-cg-apis.md`:

> **The authorization model restricts applications to modifying only their own windows unless code is injected into Dock.app, which maintains special "Universal Owner" privileges for system-wide window control.**

### Security Implications

- **Accessibility permissions are insufficient** - They grant AX API access but not CGS control over other apps
- **CGS APIs are designed for own-window management** - Cross-app control requires elevated privileges
- **System security prevents window hijacking** - Apps cannot arbitrarily manipulate other apps' windows

## Window Management Tool Implications

### Current Window Managers' Approaches

Tools like yabai, Amethyst, and Rectangle work around these restrictions via:

1. **Complete SIP Disabling** - Requires `csrutil disable` on macOS 14.5+
2. **Code Injection into Dock.app** - Gains "Universal Owner" privileges 
3. **Scripting Additions** - Broken in macOS Sequoia
4. **Accessibility APIs** - Different approach using AX framework

### Performance vs Security Trade-offs

Our research shows window management tools face increasing restrictions:
- **macOS Ventura**: Stage Manager conflicts
- **macOS Sonoma**: Further space API restrictions  
- **macOS Sequoia**: Scripting additions completely broken

## Recommendations for Symposium

### Immediate Actions

1. **Test Own Windows** - Verify CGS APIs work correctly on Symposium's own windows
2. **Implement AX Alternative** - Research Accessibility API approach for cross-app window management
3. **Architecture Decision** - Choose between:
   - Limited functionality with standard permissions
   - Advanced functionality requiring SIP disabling/code injection

### Alternative Approaches

**Option 1: Accessibility API Framework**
- Use `AXUIElement` APIs for window manipulation
- More limited but works with standard permissions
- Better compatibility across macOS versions

**Option 2: Helper Process Architecture** 
- Separate privileged helper for window operations
- User must install/authorize helper with elevated permissions
- Similar to how professional window managers work

**Option 3: Own-Window Focus**
- Concentrate on orchestrating multiple Symposium windows
- Use CGS APIs for advanced effects within our own app
- Coordinate with external tools for cross-app management

## Technical Details

### Error Codes Encountered
- **Error 1000** (`kCGErrorFailure`) - Generic failure, typically indicates insufficient privileges
- **Success with no effect** - APIs return `noErr` but changes aren't applied

### API Behavior Patterns
- **Level changes**: Appear successful but are silently ignored
- **Alpha changes**: Appear successful but are silently ignored  
- **Order changes**: Explicitly fail with error codes
- **Own windows**: Should respond to all CGS operations correctly

### Testing Framework Enhancement

We enhanced our CGS testing tool to:
- **Create test windows** - Generate Symposium windows for testing own-window control
- **Visual indicators** - Show which windows are ours (✅ Own) vs others (❌ Other)
- **Expected behavior notes** - Explain what should work where
- **Performance timing** - Measure API call response times

## Next Steps

1. **Verify Own-Window Control** - Test all CGS APIs on Symposium-created windows
2. **AX API Research** - Investigate Accessibility framework alternatives
3. **Architecture Decision** - Choose approach based on security vs functionality trade-offs
4. **User Experience Design** - Design around discovered limitations

## Conclusion

CGS APIs provide powerful window manipulation capabilities but are restricted by macOS security to prevent malicious window control. Applications can only manipulate their own windows without special privileges. Cross-application window management requires either:

- Disabling system security features (SIP)
- Code injection into system processes  
- Alternative APIs (Accessibility framework)
- Architectural changes (helper processes)

For Symposium, this means reconsidering our approach to window orchestration and choosing between security compliance and advanced functionality.