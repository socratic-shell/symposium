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
    
    private func setWindowPosition(_ windowID: CGWindowID, frame: CGRect) {
        guard let app = getAppForWindow(windowID) else { return }
        
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        if let windowArray = windows as? [AXUIElement] {
            for axWindow in windowArray {
                var windowIDRef: CFTypeRef?
                AXUIElementCopyAttributeValue(axWindow, "AXWindowID" as CFString, &windowIDRef)
                
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
    
    private func requestAccessibilityPermission() {
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