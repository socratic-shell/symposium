# macOS Core Graphics Window Management APIs

This document covers private macOS APIs from the SkyLight/Core Graphics framework for advanced window manipulation. These are undocumented, private APIs that Apple uses internally for window management.

## ⚠️ Important Warnings

- These are **private APIs** - no guarantee they'll work in future macOS versions
- **App Store rejection** - Apps using these APIs will be rejected from the App Store
- **Security implications** - May trigger system security warnings
- **Stability risks** - Improper use can affect system stability
- Use only for development tools, utilities, or research purposes

## Overview

macOS manages windows through a sophisticated layering and ordering system. The SkyLight framework provides low-level access to this system, allowing fine-grained control over window visibility, positioning, and behavior.

## Core Concepts

### Window Levels (Z-Order Layering)

Windows exist at different "levels" - imagine layers stacked on top of each other. Higher levels appear above lower levels.

```objc
// Standard macOS window levels (from lowest to highest)
kCGBackstopMenuLevel        = -20    // Behind everything
kCGNormalWindowLevel        = 0      // Regular app windows  
kCGFloatingWindowLevel      = 3      // Utility panels
kCGTornOffMenuWindowLevel   = 3      // Detached menus
kCGModalPanelWindowLevel    = 8      // Modal dialogs
kCGUtilityWindowLevel       = 19     // Inspector panels
kCGDockWindowLevel          = 20     // Dock
kCGMainMenuWindowLevel      = 24     // Menu bar
kCGStatusWindowLevel        = 25     // Status bar items
kCGPopUpMenuWindowLevel     = 101    // Pop-up menus
kCGOverlayWindowLevel       = 102    // Screen overlays
kCGHelpWindowLevel          = 200    // Help tooltips
kCGDraggingWindowLevel      = 500    // Items being dragged
kCGScreenSaverWindowLevel   = 1000   // Screen saver
kCGAssistiveTechHighWindowLevel = 1500 // Accessibility tools
kCGCursorWindowLevel        = 2147483630 // Mouse cursor
kCGMaximumWindowLevel       = 2147483631 // Absolute maximum
```

### Window Ordering States

Windows can be in different ordering states:

1. **Ordered In**: Participates in the display system, can be visible
2. **Ordered Out**: Completely removed from display system, not rendered

## Key API Functions

### Connection Management

```objc
// Get connection to the window server
extern CGSConnection CGSMainConnectionID(void);
```

### Window Level Management

```objc
// Set window level (z-order)
extern OSStatus CGSSetWindowLevel(
    CGSConnection connection,
    CGSWindowID windowID, 
    CGSWindowLevel level
);

// Get current window level
extern OSStatus CGSGetWindowLevel(
    CGSConnection connection,
    CGSWindowID windowID,
    CGSWindowLevel *level
);
```

### Window Ordering

```objc
// Window ordering modes
typedef enum {
    kCGSOrderAbove = 1,    // Order above specified window
    kCGSOrderBelow = -1,   // Order below specified window
    kCGSOrderOut = 0       // Remove from display entirely
} CGSWindowOrderingMode;

// Change window ordering
extern OSStatus CGSOrderWindow(
    CGSConnection connection,
    CGSWindowID windowID,
    CGSWindowOrderingMode ordering,
    CGSWindowID relativeToWindow
);
```

### Transparency Control

```objc
// Set window transparency (0.0 = invisible, 1.0 = opaque)
extern OSStatus CGSSetWindowAlpha(
    CGSConnection connection,
    CGSWindowID windowID,
    float alpha
);

// Get current alpha value
extern OSStatus CGSGetWindowAlpha(
    CGSConnection connection,
    CGSWindowID windowID,
    float *alpha
);
```

### Window Property Management

```objc
// Set various window properties
extern OSStatus CGSSetWindowProperty(
    CGSConnection connection,
    CGSWindowID windowID,
    CFStringRef property,
    CFTypeRef value
);

// Common properties
kCGSWindowPropertyIsOpaque
kCGSWindowPropertyBackdrop
kCGSWindowPropertyShadow
```

## Getting Window IDs

### From NSWindow (Your Own App)

```objc
NSWindow *window = // your window
CGSWindowID windowID = (CGSWindowID)[window windowNumber];
```

### From System Window List

```objc
// Get all on-screen windows
CFArrayRef windowList = CGWindowListCopyWindowInfo(
    kCGWindowListOptionOnScreenOnly, 
    kCGNullWindowID
);

// Iterate through windows
for (int i = 0; i < CFArrayGetCount(windowList); i++) {
    CFDictionaryRef windowDict = CFArrayGetValueAtIndex(windowList, i);
    
    // Get window ID
    CFNumberRef windowIDRef = CFDictionaryGetValue(windowDict, kCGWindowNumber);
    CGSWindowID windowID;
    CFNumberGetValue(windowIDRef, kCFNumberIntType, &windowID);
    
    // Get other properties
    CFStringRef ownerName = CFDictionaryGetValue(windowDict, kCGWindowOwnerName);
    CFStringRef windowName = CFDictionaryGetValue(windowDict, kCGWindowName);
}

CFRelease(windowList);
```

## Practical Examples

### Complete Window Hiding

```objc
// Method 1: Order out (most thorough)
void hideWindowCompletely(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    CGSOrderWindow(conn, windowID, kCGSOrderOut, 0);
}

// Method 2: Push behind everything  
void hideWindowBehind(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    CGSSetWindowLevel(conn, windowID, kCGMinimumWindowLevel - 1000);
}

// Method 3: Make transparent
void hideWindowTransparent(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    CGSSetWindowAlpha(conn, windowID, 0.0);
}
```

### Window Restoration

```objc
void restoreWindow(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    
    // Restore ordering
    CGSOrderWindow(conn, windowID, kCGSOrderAbove, 0);
    
    // Restore normal level
    CGSSetWindowLevel(conn, windowID, kCGNormalWindowLevel);
    
    // Restore opacity
    CGSSetWindowAlpha(conn, windowID, 1.0);
}
```

### Advanced Window Positioning

```objc
void makeWindowFloat(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    
    // Float above normal windows but below system UI
    CGSSetWindowLevel(conn, windowID, kCGFloatingWindowLevel);
}

void makeWindowSystemLevel(CGSWindowID windowID) {
    CGSConnection conn = CGSMainConnectionID();
    
    // Place at system level (above dock, below menubar)
    CGSSetWindowLevel(conn, windowID, kCGStatusWindowLevel);
}
```

### Finding and Hiding Specific Windows

```objc
void hideWindowsByAppName(NSString *appName) {
    CFArrayRef windowList = CGWindowListCopyWindowInfo(
        kCGWindowListOptionOnScreenOnly, 
        kCGNullWindowID
    );
    
    CGSConnection conn = CGSMainConnectionID();
    
    for (int i = 0; i < CFArrayGetCount(windowList); i++) {
        CFDictionaryRef windowDict = CFArrayGetValueAtIndex(windowList, i);
        
        CFStringRef ownerName = CFDictionaryGetValue(windowDict, kCGWindowOwnerName);
        if (ownerName && CFStringCompare(ownerName, (__bridge CFStringRef)appName, 0) == kCFCompareEqualTo) {
            CFNumberRef windowIDRef = CFDictionaryGetValue(windowDict, kCGWindowNumber);
            CGSWindowID windowID;
            CFNumberGetValue(windowIDRef, kCFNumberIntType, &windowID);
            
            // Hide this window
            CGSOrderWindow(conn, windowID, kCGSOrderOut, 0);
        }
    }
    
    CFRelease(windowList);
}
```

### Window Monitoring

```objc
void printAllWindows() {
    CFArrayRef windowList = CGWindowListCopyWindowInfo(
        kCGWindowListOptionAll, 
        kCGNullWindowID
    );
    
    for (int i = 0; i < CFArrayGetCount(windowList); i++) {
        CFDictionaryRef windowDict = CFArrayGetValueAtIndex(windowList, i);
        
        CFNumberRef windowIDRef = CFDictionaryGetValue(windowDict, kCGWindowNumber);
        CFStringRef ownerName = CFDictionaryGetValue(windowDict, kCGWindowOwnerName);
        CFStringRef windowName = CFDictionaryGetValue(windowDict, kCGWindowName);
        CFNumberRef levelRef = CFDictionaryGetValue(windowDict, kCGWindowLevel);
        
        int windowID, level;
        CFNumberGetValue(windowIDRef, kCFNumberIntType, &windowID);
        CFNumberGetValue(levelRef, kCFNumberIntType, &level);
        
        NSLog(@"Window ID: %d, Owner: %@, Name: %@, Level: %d", 
              windowID,
              ownerName ? (__bridge NSString*)ownerName : @"(none)",
              windowName ? (__bridge NSString*)windowName : @"(none)",
              level);
    }
    
    CFRelease(windowList);
}
```

## Comparison of Hiding Methods

| Method | Resource Usage | Detectability | Reversibility | Use Case |
|--------|---------------|---------------|---------------|----------|
| `CGSOrderWindow(..., kCGSOrderOut, 0)` | Minimal | Low | Easy | Complete hiding |
| `CGSSetWindowLevel(..., very_low_level)` | Low | Medium | Easy | Push behind |
| `CGSSetWindowAlpha(..., 0.0)` | High | High | Easy | Temporary hiding |
| Move off-screen | High | High | Easy | Simple hiding |

## Error Handling

```objc
OSStatus result = CGSSetWindowLevel(conn, windowID, level);
switch (result) {
    case kCGErrorSuccess:
        // Success
        break;
    case kCGErrorInvalidConnection:
        NSLog(@"Invalid connection to window server");
        break;
    case kCGErrorIllegalArgument:
        NSLog(@"Invalid window ID or level");
        break;
    default:
        NSLog(@"Unknown error: %d", result);
        break;
}
```

## Building and Linking

### Compiler Flags
```bash
# Link against required frameworks
-framework ApplicationServices -framework CoreGraphics
```

### Header Declarations
```objc
// Add to your header file or implementation
extern CGSConnection CGSMainConnectionID(void);
extern OSStatus CGSSetWindowLevel(CGSConnection connection, CGSWindowID window, CGSWindowLevel level);
extern OSStatus CGSOrderWindow(CGSConnection connection, CGSWindowID window, CGSWindowOrderingMode ordering, CGSWindowID relativeToWindow);
extern OSStatus CGSSetWindowAlpha(CGSConnection connection, CGSWindowID window, float alpha);
```

## Alternative Approaches

For legitimate use cases, consider these documented alternatives:

### Accessibility APIs
```objc
// Minimize window via accessibility
AXUIElementRef windowElement = // get window element
AXUIElementSetAttributeValue(windowElement, kAXMinimizedAttribute, kCFBooleanTrue);
```

### Standard NSWindow Methods (Own App Only)
```objc
[window setIsVisible:NO];
[window orderOut:nil];
[window miniaturize:nil];
```

### AppleScript/JXA
```applescript
tell application "System Events"
    tell process "AppName"
        set visible to false
    end tell
end tell
```

## Security Considerations

1. **Code Signing**: May require specific entitlements
2. **System Integrity Protection**: Some operations may be restricted
3. **User Permissions**: May require accessibility permissions
4. **Gatekeeper**: Unsigned apps using private APIs may trigger warnings

## Version Compatibility

These APIs have been relatively stable since macOS 10.5, but:
- Function signatures may change
- New security restrictions may be added
- Performance characteristics may vary
- Always test on target macOS versions

## Best Practices

1. **Graceful Fallbacks**: Always have fallback methods for when APIs fail
2. **Permission Checks**: Verify accessibility permissions before use
3. **Error Handling**: Check return values and handle failures
4. **Resource Cleanup**: Release window lists and other CF objects
5. **Testing**: Test thoroughly across macOS versions
6. **User Experience**: Don't interfere with system behavior unexpectedly

---

*Remember: These are private, undocumented APIs. Use responsibly and only when necessary. Consider documented alternatives first.*