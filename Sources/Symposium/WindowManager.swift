import AppKit
import Cocoa
import SwiftUI
import Accessibility

class WindowManager: ObservableObject {
    struct WindowInfo: Identifiable {
        let id: CGWindowID
        let title: String
        let appName: String
        var originalFrame: CGRect?
        
        var displayName: String {
            if !title.isEmpty {
                return "\(appName): \(title)"
            } else {
                // For common apps, provide better fallback names
                switch appName {
                case "Code":
                    return "\(appName): Editor Window"
                case "Terminal":
                    return "\(appName): Terminal Window"
                case "Safari":
                    return "\(appName): Browser Window"
                case "Chrome", "Google Chrome":
                    return "\(appName): Browser Window"
                case "Firefox":
                    return "\(appName): Browser Window"
                case "Finder":
                    return "\(appName): Finder Window"
                case "TextEdit":
                    return "\(appName): Document"
                default:
                    return "\(appName): Window"
                }
            }
        }
    }
    
    @Published var allWindows: [WindowInfo] = []
    @Published var stackedWindows: [WindowInfo] = []
    @Published var currentStackIndex: Int = 0
    @Published var hasAccessibilityPermission: Bool = false
    @Published var lastOperationMessage: String = ""
    
    init() {
        checkAccessibilityPermission()
        refreshWindowList()
        lastOperationMessage = hasAccessibilityPermission ? "Ready to manage windows" : "Accessibility permission required"
    }
    
    func checkAccessibilityPermission() {
        hasAccessibilityPermission = AXIsProcessTrusted()
        print("🔐 Accessibility permission status: \(hasAccessibilityPermission)")
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
            
            // Try to get a better window title using multiple approaches
            var title = dict[kCGWindowName as String] as? String ?? ""
            
            // If title is still empty, try to get it via Accessibility API
            if title.isEmpty {
                title = getWindowTitleViaAccessibility(windowID: windowID) ?? ""
            }
            
            let frame = CGRect(
                x: bounds["X"] ?? 0,
                y: bounds["Y"] ?? 0,
                width: bounds["Width"] ?? 0,
                height: bounds["Height"] ?? 0
            )
            
            // Skip our own window and already stacked windows
            if appName == "Symposium" || stackedWindows.contains(where: { $0.id == windowID }) {
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
        print("🔍 Adding window to stack: \(window.displayName) (ID: \(window.id))")
        
        // Check permission first
        checkAccessibilityPermission()
        if !hasAccessibilityPermission {
            lastOperationMessage = "❌ Accessibility permission required to move windows"
            requestAccessibilityPermission()
            return
        }
        
        var windowWithFrame = window
        windowWithFrame.originalFrame = getWindowFrame(window.id)
        
        print("📍 Original frame: \(windowWithFrame.originalFrame?.debugDescription ?? "nil")")
        
        if let firstWindow = stackedWindows.first,
           let targetFrame = firstWindow.originalFrame {
            print("🎯 Moving window to target frame: \(targetFrame)")
            let success = setWindowPosition(window.id, frame: targetFrame)
            print(success ? "✅ Window move successful" : "❌ Window move failed")
            lastOperationMessage = success ? 
                "✅ Added \(window.appName) to stack" : 
                "❌ Failed to move \(window.appName) window"
        } else {
            print("📌 First window in stack - keeping original position")
            lastOperationMessage = "📌 \(window.appName) is now the first window in stack"
        }
        
        stackedWindows.append(windowWithFrame)
        refreshWindowList()
        
        // Focus the newly added window
        currentStackIndex = stackedWindows.count - 1
        print("🔄 Focusing window...")
        focusWindow(window.id)
        print("📚 Stack now contains \(stackedWindows.count) windows")
    }
    
    func removeFromStack(_ window: WindowInfo) {
        print("🗑️ Removing window from stack: \(window.displayName) (ID: \(window.id))")
        
        guard let index = stackedWindows.firstIndex(where: { $0.id == window.id }) else { return }
        
        let removed = stackedWindows.remove(at: index)
        
        // Restore original position if we have it
        if let originalFrame = removed.originalFrame {
            print("↩️ Restoring original position: \(originalFrame)")
            let success = setWindowPosition(removed.id, frame: originalFrame)
            print(success ? "✅ Position restored successfully" : "❌ Failed to restore position")
        }
        
        // Adjust current index
        if currentStackIndex >= stackedWindows.count && !stackedWindows.isEmpty {
            currentStackIndex = stackedWindows.count - 1
        }
        
        print("📚 Stack now contains \(stackedWindows.count) windows")
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
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        if let windowArray = windows as? [AXUIElement] {
            // Find and raise our specific window
            for axWindow in windowArray {
                var windowIDRef: CFTypeRef?
                AXUIElementCopyAttributeValue(axWindow, "AXWindowID" as CFString, &windowIDRef)
                
                if let id = windowIDRef as? CGWindowID, id == windowID {
                    AXUIElementPerformAction(axWindow, kAXRaiseAction as CFString)
                    AXUIElementSetAttributeValue(axWindow, kAXMainAttribute as CFString, true as CFBoolean)
                    break
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
    
    private func setWindowPosition(_ windowID: CGWindowID, frame: CGRect) -> Bool {
        print("🔧 Attempting to set position for window \(windowID)")
        
        // Check accessibility permission first
        let trusted = AXIsProcessTrusted()
        print("🔐 Accessibility trusted: \(trusted)")
        if !trusted {
            print("❌ No accessibility permission - cannot move windows")
            return false
        }
        
        guard let app = getAppForWindow(windowID) else { 
            print("❌ Could not find app for window \(windowID)")
            return false
        }
        
        print("🏃 Found app: \(app.localizedName ?? app.bundleIdentifier ?? "Unknown") (PID: \(app.processIdentifier))")
        
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        let windowsResult = AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        if windowsResult != .success {
            print("❌ Failed to get windows list, error: \(windowsResult.rawValue)")
            return false
        }
        
        guard let windowArray = windows as? [AXUIElement] else {
            print("❌ Windows list is not an array")
            return false
        }
        
        print("🪟 Found \(windowArray.count) windows in app")
        
        for (index, axWindow) in windowArray.enumerated() {
            var windowIDRef: CFTypeRef?
            let idResult = AXUIElementCopyAttributeValue(axWindow, "AXWindowID" as CFString, &windowIDRef)
            
            if idResult != .success {
                print("⚠️ Window \(index): Could not get window ID, error: \(idResult.rawValue)")
                continue
            }
            
            if let id = windowIDRef as? CGWindowID {
                print("🔎 Window \(index): ID = \(id) (looking for \(windowID))")
                
                if id == windowID {
                    print("🎯 Found matching window!")
                    
                    // Set position
                    var position = CGPoint(x: frame.origin.x, y: frame.origin.y)
                    let positionValue = AXValueCreate(.cgPoint, &position)!
                    let posResult = AXUIElementSetAttributeValue(axWindow, kAXPositionAttribute as CFString, positionValue)
                    print("📍 Set position result: \(posResult.rawValue)")
                    
                    // Set size
                    var size = CGSize(width: frame.width, height: frame.height)
                    let sizeValue = AXValueCreate(.cgSize, &size)!
                    let sizeResult = AXUIElementSetAttributeValue(axWindow, kAXSizeAttribute as CFString, sizeValue)
                    print("📏 Set size result: \(sizeResult.rawValue)")
                    
                    return posResult == .success && sizeResult == .success
                }
            } else {
                print("⚠️ Window \(index): Window ID is not CGWindowID type")
            }
        }
        
        print("❌ No matching window found in app")
        return false
    }
    
    private func getAppForWindow(_ windowID: CGWindowID) -> NSRunningApplication? {
        let windowList = CGWindowListCopyWindowInfo(.optionIncludingWindow, windowID) as? [[String: Any]]
        guard let dict = windowList?.first,
              let pid = dict[kCGWindowOwnerPID as String] as? pid_t else {
            return nil
        }
        
        return NSWorkspace.shared.runningApplications.first { $0.processIdentifier == pid }
    }
    
    private func getWindowTitleViaAccessibility(windowID: CGWindowID) -> String? {
        guard let app = getAppForWindow(windowID) else { return nil }
        
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        if let windowArray = windows as? [AXUIElement] {
            for axWindow in windowArray {
                var windowIDRef: CFTypeRef?
                AXUIElementCopyAttributeValue(axWindow, "AXWindowID" as CFString, &windowIDRef)
                
                if let id = windowIDRef as? CGWindowID, id == windowID {
                    var titleRef: CFTypeRef?
                    AXUIElementCopyAttributeValue(axWindow, kAXTitleAttribute as CFString, &titleRef)
                    
                    if let title = titleRef as? String, !title.isEmpty {
                        return title
                    }
                    
                    // Also try AXDescription as fallback
                    var descRef: CFTypeRef?
                    AXUIElementCopyAttributeValue(axWindow, kAXDescriptionAttribute as CFString, &descRef)
                    
                    if let desc = descRef as? String, !desc.isEmpty {
                        return desc
                    }
                    
                    break
                }
            }
        }
        
        return nil
    }
    
    func requestAccessibilityPermission() {
        let trusted = AXIsProcessTrusted()
        if !trusted {
            let alert = NSAlert()
            alert.messageText = "Accessibility Permission Required"
            alert.informativeText = "Symposium needs accessibility permission to manage windows."
            alert.addButton(withTitle: "Open System Preferences")
            alert.addButton(withTitle: "Cancel")
            
            if alert.runModal() == .alertFirstButtonReturn {
                NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!)
            }
        }
    }
}