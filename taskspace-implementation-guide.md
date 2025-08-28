# Taskspace Window Multiplexer - Implementation Guide

## Project Overview
Building a macOS window stacking manager that allows users to stack multiple application windows (VS Code, Terminal, etc.) in the same position and quickly switch between them. This is a proof of concept that will evolve into a full window multiplexer.

## Initial Setup

### 1. Create Project Structure
```bash
mkdir TaskspaceProof && cd TaskspaceProof
swift package init --type executable --name Taskspace
```

### 2. Update Package.swift
Replace the generated Package.swift with:

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Taskspace",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "Taskspace", targets: ["Taskspace"])
    ],
    targets: [
        .executableTarget(
            name: "Taskspace",
            dependencies: []
        )
    ]
)
```

## Core Implementation Files

### Sources/Taskspace/main.swift
```swift
import AppKit
import SwiftUI

@main
struct TaskspaceApp: App {
    @StateObject private var windowManager = WindowManager()
    
    var body: some Scene {
        WindowGroup {
            ContentView(windowManager: windowManager)
                .frame(width: 400, height: 600)
        }
    }
}
```

### Sources/Taskspace/WindowManager.swift
```swift
import Cocoa
import Accessibility

class WindowManager: ObservableObject {
    struct WindowInfo: Identifiable {
        let id: CGWindowID
        let title: String
        let appName: String
        var originalFrame: CGRect?
        
        var displayName: String {
            "\(appName): \(title.isEmpty ? "Untitled" : title)"
        }
    }
    
    @Published var allWindows: [WindowInfo] = []
    @Published var stackedWindows: [WindowInfo] = []
    @Published var currentStackIndex: Int = 0
    
    init() {
        requestAccessibilityPermission()
        refreshWindowList()
    }
    
    // MARK: - Core Functions
    
    func refreshWindowList() {
        let options = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
        let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
        
        allWindows = windowList.compactMap { dict in
            guard let windowID = dict[kCGWindowNumber as String] as? CGWindowID,
                  let appName = dict[kCGWindowOwnerName as String] as? String,
                  dict[kCGWindowLayer as String] as? Int == 0, // Normal windows only
                  let bounds = dict[kCGWindowBounds as String] as? [String: CGFloat] else {
                return nil
            }
            
            let title = dict[kCGWindowName as String] as? String ?? ""
            let frame = CGRect(
                x: bounds["X"] ?? 0,
                y: bounds["Y"] ?? 0,
                width: bounds["Width"] ?? 0,
                height: bounds["Height"] ?? 0
            )
            
            // Skip our own window and already stacked windows
            if appName == "Taskspace" || stackedWindows.contains(where: { $0.id == windowID }) {
                return nil
            }
            
            return WindowInfo(
                id: windowID,
                title: title,
                appName: appName,
                originalFrame: frame
            )
        }
    }
    
    func addToStack(_ window: WindowInfo) {
        var windowWithFrame = window
        windowWithFrame.originalFrame = getWindowFrame(window.id)
        
        if let firstWindow = stackedWindows.first,
           let targetFrame = firstWindow.originalFrame {
            // Move window to same position as first in stack
            setWindowPosition(window.id, frame: targetFrame)
        }
        
        stackedWindows.append(windowWithFrame)
        refreshWindowList()
        
        // Focus the newly added window
        currentStackIndex = stackedWindows.count - 1
        focusWindow(window.id)
    }
    
    func removeFromStack(_ window: WindowInfo) {
        guard let index = stackedWindows.firstIndex(where: { $0.id == window.id }) else { return }
        
        let removed = stackedWindows.remove(at: index)
        
        // Restore original position if we have it
        if let originalFrame = removed.originalFrame {
            setWindowPosition(removed.id, frame: originalFrame)
        }
        
        // Adjust current index
        if currentStackIndex >= stackedWindows.count && !stackedWindows.isEmpty {
            currentStackIndex = stackedWindows.count - 1
        }
        
        refreshWindowList()
    }
    
    func nextWindow() {
        guard !stackedWindows.isEmpty else { return }
        currentStackIndex = (currentStackIndex + 1) % stackedWindows.count
        focusWindow(stackedWindows[currentStackIndex].id)
    }
    
    func previousWindow() {
        guard !stackedWindows.isEmpty else { return }
        currentStackIndex = currentStackIndex == 0 ? stackedWindows.count - 1 : currentStackIndex - 1
        focusWindow(stackedWindows[currentStackIndex].id)
    }
    
    // MARK: - Window Manipulation
    
    private func focusWindow(_ windowID: CGWindowID) {
        // Find the app that owns this window
        guard let app = getAppForWindow(windowID) else { return }
        
        // Bring app to front
        app.activate(options: .activateIgnoringOtherApps)
        
        // Use Accessibility API to raise specific window
        if let axApp = AXUIElementCreateApplication(app.processIdentifier) {
            var windows: CFTypeRef?
            AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
            
            if let windowArray = windows as? [AXUIElement] {
                // Find and raise our specific window
                for axWindow in windowArray {
                    var windowIDRef: CFTypeRef?
                    AXUIElementCopyAttributeValue(axWindow, kAXWindowIDAttribute as CFString, &windowIDRef)
                    
                    if let id = windowIDRef as? CGWindowID, id == windowID {
                        AXUIElementPerformAction(axWindow, kAXRaiseAction as CFString)
                        AXUIElementSetAttributeValue(axWindow, kAXMainAttribute as CFString, true as CFBoolean)
                        break
                    }
                }
            }
        }
    }
    
    private func getWindowFrame(_ windowID: CGWindowID) -> CGRect? {
        let windowList = CGWindowListCopyWindowInfo(.optionIncludingWindow, windowID) as? [[String: Any]]
        guard let dict = windowList?.first,
              let bounds = dict[kCGWindowBounds as String] as? [String: CGFloat] else {
            return nil
        }
        
        return CGRect(
            x: bounds["X"] ?? 0,
            y: bounds["Y"] ?? 0,
            width: bounds["Width"] ?? 0,
            height: bounds["Height"] ?? 0
        )
    }
    
    private func setWindowPosition(_ windowID: CGWindowID, frame: CGRect) {
        guard let app = getAppForWindow(windowID) else { return }
        
        if let axApp = AXUIElementCreateApplication(app.processIdentifier) {
            var windows: CFTypeRef?
            AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
            
            if let windowArray = windows as? [AXUIElement] {
                for axWindow in windowArray {
                    var windowIDRef: CFTypeRef?
                    AXUIElementCopyAttributeValue(axWindow, kAXWindowIDAttribute as CFString, &windowIDRef)
                    
                    if let id = windowIDRef as? CGWindowID, id == windowID {
                        // Set position
                        var position = CGPoint(x: frame.origin.x, y: frame.origin.y)
                        let positionValue = AXValueCreate(.cgPoint, &position)
                        AXUIElementSetAttributeValue(axWindow, kAXPositionAttribute as CFString, positionValue!)
                        
                        // Set size
                        var size = CGSize(width: frame.width, height: frame.height)
                        let sizeValue = AXValueCreate(.cgSize, &size)
                        AXUIElementSetAttributeValue(axWindow, kAXSizeAttribute as CFString, sizeValue!)
                        break
                    }
                }
            }
        }
    }
    
    private func getAppForWindow(_ windowID: CGWindowID) -> NSRunningApplication? {
        let windowList = CGWindowListCopyWindowInfo(.optionIncludingWindow, windowID) as? [[String: Any]]
        guard let dict = windowList?.first,
              let pid = dict[kCGWindowOwnerPID as String] as? pid_t else {
            return nil
        }
        
        return NSWorkspace.shared.runningApplications.first { $0.processIdentifier == pid }
    }
    
    private func requestAccessibilityPermission() {
        let trusted = AXIsProcessTrusted()
        if !trusted {
            let alert = NSAlert()
            alert.messageText = "Accessibility Permission Required"
            alert.informativeText = "Taskspace needs accessibility permission to manage windows."
            alert.addButton(withTitle: "Open System Preferences")
            alert.addButton(withTitle: "Cancel")
            
            if alert.runModal() == .alertFirstButtonReturn {
                NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!)
            }
        }
    }
}
```

### Sources/Taskspace/ContentView.swift
```swift
import SwiftUI

struct ContentView: View {
    @ObservedObject var windowManager: WindowManager
    
    var body: some View {
        VStack(spacing: 20) {
            // Stack section
            VStack {
                Text("Stack (\(windowManager.stackedWindows.count) windows)")
                    .font(.headline)
                
                ScrollView {
                    ForEach(windowManager.stackedWindows) { window in
                        HStack {
                            Text(window.displayName)
                                .foregroundColor(
                                    windowManager.currentStackIndex == 
                                    windowManager.stackedWindows.firstIndex(where: { $0.id == window.id }) 
                                    ? .blue : .primary
                                )
                            Spacer()
                            Button("Remove") {
                                windowManager.removeFromStack(window)
                            }
                        }
                        .padding(.horizontal)
                    }
                }
                .frame(height: 150)
                .border(Color.gray)
                
                HStack {
                    Button("Previous") {
                        windowManager.previousWindow()
                    }
                    .disabled(windowManager.stackedWindows.isEmpty)
                    
                    Button("Next") {
                        windowManager.nextWindow()
                    }
                    .disabled(windowManager.stackedWindows.isEmpty)
                }
            }
            
            Divider()
            
            // Available windows section
            VStack {
                HStack {
                    Text("Available Windows")
                        .font(.headline)
                    Spacer()
                    Button("Refresh") {
                        windowManager.refreshWindowList()
                    }
                }
                
                ScrollView {
                    ForEach(windowManager.allWindows) { window in
                        HStack {
                            Text(window.displayName)
                            Spacer()
                            Button("Add to Stack") {
                                windowManager.addToStack(window)
                            }
                        }
                        .padding(.horizontal)
                    }
                }
            }
        }
        .padding()
    }
}
```

## Building and Running

```bash
# Build and run in debug mode
swift build
swift run

# Build release version
swift build -c release

# Run release version
./.build/release/Taskspace
```

## Required Permissions

**IMPORTANT**: The app needs Accessibility permission to control windows.
- On first run, it will prompt for permission
- Grant in: System Preferences → Security & Privacy → Privacy → Accessibility
- You may need to add the built executable manually

## Troubleshooting

### Issue: Accessibility Permission Not Working

1. Create Info.plist file:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.taskspace.app</string>
    <key>CFBundleName</key>
    <string>Taskspace</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

2. Create app bundle and sign:
```bash
# After building
mkdir -p .build/debug/Taskspace.app/Contents/MacOS
cp .build/debug/Taskspace .build/debug/Taskspace.app/Contents/MacOS/
cp Info.plist .build/debug/Taskspace.app/Contents/
codesign -s - --deep --force .build/debug/Taskspace.app
```

### Issue: Windows Not Focusing Properly

Add this alternative focus method to WindowManager.swift:
```swift
private func focusWindowViaAppleScript(_ windowID: CGWindowID) {
    guard let app = getAppForWindow(windowID) else { return }
    
    let script = """
        tell application "System Events"
            set frontmost of first process whose unix id is \(app.processIdentifier) to true
        end tell
    """
    
    if let scriptObject = NSAppleScript(source: script) {
        scriptObject.executeAndReturnError(nil)
    }
}
```

## Testing Instructions

1. **Launch the app** using `swift run`
2. **Open multiple VS Code windows** (or Terminal, Safari, etc.)
3. **Click "Add to Stack"** on 2-3 windows
4. **Observe** that windows move to the same position
5. **Click "Next"/"Previous"** to cycle through them
6. **Click "Remove from Stack"** to restore original positions

## Implementation Notes

### Current Features
- Lists all open windows on screen
- Add windows to a single stack
- Windows in stack move to same position
- Next/Previous buttons cycle through stacked windows
- Remove from stack restores original position
- Visual indicator shows current window in stack

### Known Limitations
- Only one stack supported (by design for proof of concept)
- Window focusing may be inconsistent
- No keyboard shortcuts yet
- No window previews
- Stack not persisted between app launches

## Next Steps

### Phase 1: Stabilize Core
- [ ] Fix window focusing reliability
- [ ]] Add error handling for permission failures
- [ ] Improve window detection after spawning
- [ ] Add keyboard shortcuts (Cmd+] for next, Cmd+[ for previous)

### Phase 2: Enhanced UI
- [ ] Add window preview thumbnails using ScreenCaptureKit
- [ ] Menu bar app instead of window
- [ ] Persist stack between launches
- [ ] Multiple named stacks support

### Phase 3: IDE Integration
- [ ] WebSocket server for IPC
- [ ] VS Code extension to add windows programmatically
- [ ] Command-line interface

## Quick Command Reference

```bash
# Build
swift build

# Run
swift run

# Clean
swift package clean

# Build optimized
swift build -c release

# Reset accessibility permissions
tccutil reset Accessibility com.taskspace.app
```

## Success Criteria

- ✅ Windows can be added to a stack
- ✅ Windows move to same position when stacked
- ✅ Can cycle through stacked windows
- ✅ Windows restore position when removed
- ⚠️ Focus switching needs improvement
- ⚠️ Test with VS Code, Terminal, and browsers

## Tips for Claude Code

1. **Start simple**: Get the basic proof of concept working first
2. **Test frequently**: Run the app after each change
3. **Debug permissions**: If windows won't move, check Accessibility permissions
4. **Use print statements**: Add debugging output to understand window IDs and state
5. **Test with multiple apps**: Don't just test with VS Code - try Terminal, Safari, etc.

## Resources

- [Apple Accessibility API](https://developer.apple.com/documentation/applicationservices/kaxwindowsattribute)
- [CGWindow Reference](https://developer.apple.com/documentation/coregraphics/quartz_window_services)
- [Swift Package Manager](https://swift.org/package-manager/)
- [SwiftUI Documentation](https://developer.apple.com/documentation/swiftui)

---

Good luck implementing Taskspace! Start with getting the basic proof of concept running, then iterate from there.