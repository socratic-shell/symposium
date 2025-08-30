# Why We Rejected Core Graphics Services APIs

*The story of exploring, testing, and ultimately rejecting CGS APIs for Symposium's window management*

## Executive Summary

We extensively researched and tested Core Graphics Services (CGS) APIs as a potential foundation for Symposium's cross-application window management capabilities. Through empirical testing and security analysis, we discovered fundamental limitations that make CGS APIs unsuitable for Symposium's core mission of orchestrating windows from different applications.

**Key Finding:** CGS APIs are restricted by macOS security to only manipulate an application's own windows, making them useless for cross-application window management.

## The Appeal of CGS APIs

CGS APIs initially appeared very promising for Symposium because they offer:

- **Powerful window manipulation** - Complete control over window positioning, transparency, and z-order
- **Performance advantages** - Direct communication with WindowServer bypasses higher-level abstractions
- **Advanced features** - Capabilities not available through public NSWindow APIs
- **Used by professional tools** - Window managers like yabai rely on these APIs

The research in our supporting documents showed that CGS APIs provide exactly the fine-grained control we wanted for window stacking and manipulation.

## Our Experimental Approach

To properly evaluate CGS APIs, we:

1. **Built a comprehensive testing tool** - Created a CGS API testing interface within Symposium
2. **Tested real-world scenarios** - Experimented with Chrome, Terminal, and other target applications  
3. **Implemented all major operations** - Order In/Out, level changes, transparency control
4. **Added performance monitoring** - Measured API response times and success rates
5. **Created test windows** - Generated Symposium windows to test own-window control

## Critical Discovery: The Security Wall

Our testing revealed a fundamental security restriction in macOS:

### What We Observed
- **Chrome window tests**: Level and alpha changes returned "success" but had no visual effect
- **Terminal window tests**: Same pattern - success codes with no actual changes
- **Order operations**: Explicitly failed with error 1000 (`kCGErrorFailure`)
- **Own windows**: All operations worked correctly on Symposium-created windows

### The Root Cause
Research revealed that macOS restricts CGS window manipulation through an authorization model:

> **The authorization model restricts applications to modifying only their own windows unless code is injected into Dock.app, which maintains special "Universal Owner" privileges for system-wide window control.**

This security model prevents window hijacking attacks but also blocks legitimate window management applications.

## Why This Breaks Symposium's Vision

Symposium's core mission is to be a "meta-IDE" that orchestrates multiple applications:
- VS Code or other IDEs
- Terminal applications  
- Browser windows
- Development tools like IntelliJ

**CGS APIs can only control Symposium's own windows**, making them fundamentally incompatible with this vision.

## Workarounds We Considered

The research showed that existing window managers work around these restrictions via:

1. **Complete SIP Disabling** - Requires `csrutil disable` on macOS 14.5+
2. **Code Injection into Dock.app** - Gains "Universal Owner" privileges
3. **Scripting Additions** - Broken in macOS Sequoia
4. **Helper Processes** - Separate privileged components

All of these approaches require:
- **Compromising system security** (SIP disabling)
- **Complex installation procedures** (helper processes)  
- **Fragile workarounds** (code injection)
- **Version-specific maintenance** (API changes)

## The Decision to Reject CGS APIs

We rejected CGS APIs for Symposium because:

1. **Core incompatibility** - Cannot control other applications' windows
2. **Security trade-offs** - Workarounds require compromising macOS security
3. **Maintenance burden** - Private APIs change without notice
4. **User experience** - Complex installation and permission requirements

## Impact on Symposium Architecture

This decision forces us to reconsider Symposium's architecture:

### Alternative Approaches to Explore
1. **Accessibility API Framework** - More limited but works with standard permissions
2. **Coordination over Control** - Focus on communication rather than window manipulation
3. **Integration with Existing Tools** - Partner with established window managers
4. **Different Interaction Models** - Rethink how "orchestration" should work

### Lessons Learned
- **Security-first design** matters more than powerful APIs
- **Cross-application control** is increasingly restricted in modern macOS
- **User experience** should prioritize ease of use over advanced features
- **Platform limitations** must be accepted rather than worked around

## Conclusion

While CGS APIs offer impressive window manipulation capabilities, their restriction to own-window control makes them unsuitable for Symposium's cross-application orchestration goals. This exploration was valuable for understanding the technical landscape and will inform our choice of alternative approaches.

The comprehensive testing infrastructure we built and the deep understanding of macOS window management we gained will be valuable as we pivot to Accessibility APIs or other architectural approaches.

## Next Steps

1. **Research Accessibility API approach** - Investigate AXUIElement framework for cross-app control
2. **Evaluate coordination models** - Design Symposium around communication rather than control
3. **Prototype alternative architectures** - Test viability of different approaches
4. **User experience design** - Define what "orchestration" means within platform constraints