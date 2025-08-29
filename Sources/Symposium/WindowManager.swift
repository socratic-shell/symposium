import Accessibility
import AppKit
import Cocoa
import SwiftUI

class WindowManager: ObservableObject {
    struct WindowInfo: Identifiable {
        let id: CGWindowID
        let title: String
        let appName: String
        var originalFrame: CGRect?
        var isLeader: Bool = false

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
    @Published var debugLog: String = ""
    
    // Window stacking configuration
    @Published var insetPercentage: Float = 0.10 // 10% default inset
    private let minimumInset: CGFloat = 10.0
    private let maximumInset: CGFloat = 150.0
    
    // Movement tracking
    private var axObserver: AXObserver?
    private var currentLeaderWindow: WindowInfo?

    init() {
        checkAccessibilityPermission()
        refreshWindowList()
        lastOperationMessage =
            hasAccessibilityPermission
            ? "Ready to manage windows" : "Accessibility permission required"
        setupMovementObserver()
    }
    
    deinit {
        cleanupObserver()
    }

    func checkAccessibilityPermission() {
        // Use improved permission checking per macOS Sequoia research
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
        ]
        hasAccessibilityPermission = AXIsProcessTrustedWithOptions(options as CFDictionary)
        log("🔐 Accessibility permission status: \(hasAccessibilityPermission)")
    }

    // MARK: - Core Functions

    func refreshWindowList() {
        let options = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
        let windowList =
            CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []

        allWindows = windowList.compactMap { dict in
            guard let windowID = dict[kCGWindowNumber as String] as? CGWindowID,
                let appName = dict[kCGWindowOwnerName as String] as? String,
                dict[kCGWindowLayer as String] as? Int == 0,  // Normal windows only
                let bounds = dict[kCGWindowBounds as String] as? [String: CGFloat]
            else {
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
        log("🔍 Adding window to stack: \(window.displayName) (ID: \(window.id))")

        // Check permission first
        checkAccessibilityPermission()
        if !hasAccessibilityPermission {
            lastOperationMessage = "❌ Accessibility permission required to move windows"
            log("❌ No accessibility permission")
            requestAccessibilityPermission()
            return
        }

        var windowWithFrame = window
        windowWithFrame.originalFrame = getWindowFrame(window.id)

        log("📍 Original frame: \(windowWithFrame.originalFrame?.debugDescription ?? "nil")")

        if stackedWindows.isEmpty {
            // First window becomes the leader
            windowWithFrame.isLeader = true
            currentLeaderWindow = windowWithFrame
            log("👑 \(window.appName) is now the leader (first in stack)")
            lastOperationMessage = "👑 \(window.appName) is now the stack leader"
        } else {
            // New window becomes follower
            windowWithFrame.isLeader = false
            
            // Position as follower relative to current leader
            if let leaderFrame = currentLeaderWindow?.originalFrame {
                let followerFrame = calculateFollowerFrame(leaderFrame: leaderFrame)
                log("🎯 Positioning follower at: \(followerFrame)")
                let success = setWindowPosition(window.id, frame: followerFrame)
                log(success ? "✅ Follower positioned successfully" : "❌ Failed to position follower")
                
                // Update the stored frame to the follower position
                windowWithFrame.originalFrame = followerFrame
                
                lastOperationMessage = success
                    ? "✅ Added \(window.appName) as follower"
                    : "❌ Failed to position \(window.appName) as follower"
            }
        }

        stackedWindows.append(windowWithFrame)
        refreshWindowList()

        // Focus the newly added window and make it the new leader if it's not the first
        currentStackIndex = stackedWindows.count - 1
        if stackedWindows.count == 1 {
            // First window - subscribe to its movements
            subscribeToLeaderMovement(windowWithFrame)
        } else {
            // Switch leadership to the new window
            switchToLeader(windowWithFrame)
        }
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
        let window = stackedWindows[currentStackIndex]
        log("⏭️ Next window: \(window.displayName) (index \(currentStackIndex))")
        focusWindow(stackedWindows[currentStackIndex].id)
    }

    func previousWindow() {
        guard !stackedWindows.isEmpty else { return }
        currentStackIndex =
            currentStackIndex == 0 ? stackedWindows.count - 1 : currentStackIndex - 1
        let window = stackedWindows[currentStackIndex]
        log("⏮️ Previous window: \(window.displayName) (index \(currentStackIndex))")
        focusWindow(stackedWindows[currentStackIndex].id)
    }

    // MARK: - Window Manipulation

    private func focusWindow(_ windowID: CGWindowID) {
        log("🎯 Focusing window ID: \(windowID) using AeroSpace approach")

        // Find the app that owns this window
        guard let app = getAppForWindow(windowID) else {
            log("❌ Could not find app for window \(windowID)")
            return
        }

        log("🏃 Focusing app: \(app.localizedName ?? "Unknown")")
        // Bring app to front
        app.activate(options: .activateIgnoringOtherApps)

        // Use Accessibility API to get all windows for this app
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        let windowsResult = AXUIElementCopyAttributeValue(
            axApp, kAXWindowsAttribute as CFString, &windows)

        if windowsResult != .success {
            log("❌ Failed to get windows list for focusing, error: \(windowsResult.rawValue)")
            return
        }

        guard let windowArray = windows as? [AXUIElement] else {
            log("❌ Windows list is not an array for focusing")
            return
        }

        log("🪟 Searching \(windowArray.count) AX windows using _AXUIElementGetWindow")

        // Use AeroSpace's proven approach: _AXUIElementGetWindow
        for (index, axWindow) in windowArray.enumerated() {
            if let axWindowID = getWindowID(from: axWindow) {
                log("🔍 AX Window \(index): ID = \(axWindowID) (looking for \(windowID))")
                
                if axWindowID == windowID {
                    log("🎯 Found exact match using _AXUIElementGetWindow!")
                    
                    let raiseResult = AXUIElementPerformAction(axWindow, kAXRaiseAction as CFString)
                    log("📢 Raise action result: \(raiseResult.rawValue) (\(axErrorString(raiseResult)))")

                    let mainResult = AXUIElementSetAttributeValue(
                        axWindow, kAXMainAttribute as CFString, true as CFBoolean)
                    log("📢 Set main attribute result: \(mainResult.rawValue) (\(axErrorString(mainResult)))")

                    return
                }
            } else {
                log("⚠️ AX Window \(index): _AXUIElementGetWindow failed")
            }
        }

        log("❌ Could not find window to focus using _AXUIElementGetWindow")
    }

    private func getWindowFrame(_ windowID: CGWindowID) -> CGRect? {
        let windowList =
            CGWindowListCopyWindowInfo(.optionIncludingWindow, windowID) as? [[String: Any]]
        guard let dict = windowList?.first,
            let bounds = dict[kCGWindowBounds as String] as? [String: CGFloat]
        else {
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
        log("🔧 Attempting to set position for window \(windowID)")

        // Check accessibility permission first using improved method
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
        ]
        let trusted = AXIsProcessTrustedWithOptions(options as CFDictionary)
        log("🔐 Accessibility trusted: \(trusted)")
        if !trusted {
            log("❌ No accessibility permission - cannot move windows")
            return false
        }

        guard let app = getAppForWindow(windowID) else {
            log("❌ Could not find app for window \(windowID)")
            return false
        }

        log(
            "🏃 Found app: \(app.localizedName ?? app.bundleIdentifier ?? "Unknown") (PID: \(app.processIdentifier))"
        )

        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        let windowsResult = AXUIElementCopyAttributeValue(
            axApp, kAXWindowsAttribute as CFString, &windows)

        if windowsResult != .success {
            log("❌ Failed to get windows list, error: \(windowsResult.rawValue)")
            return false
        }

        guard let windowArray = windows as? [AXUIElement] else {
            log("❌ Windows list is not an array")
            return false
        }

        log("🪟 Found \(windowArray.count) windows in app")

        // Use AeroSpace's proven approach: _AXUIElementGetWindow
        for (index, axWindow) in windowArray.enumerated() {
            if let axWindowID = getWindowID(from: axWindow) {
                log("🔎 Window \(index): ID = \(axWindowID) (looking for \(windowID))")
                
                if axWindowID == windowID {
                    log("🎯 Found matching window using _AXUIElementGetWindow!")

                    // Check if window supports position/size changes
                    var positionValue: CFTypeRef?
                    let canReadPos = AXUIElementCopyAttributeValue(
                        axWindow, kAXPositionAttribute as CFString, &positionValue)
                    var sizeValue: CFTypeRef?
                    let canReadSize = AXUIElementCopyAttributeValue(
                        axWindow, kAXSizeAttribute as CFString, &sizeValue)

                    log(
                        "🔍 Window attribute check - Position readable: \(canReadPos == .success), Size readable: \(canReadSize == .success)"
                    )

                    // Set position
                    var position = CGPoint(x: frame.origin.x, y: frame.origin.y)
                    let newPositionValue = AXValueCreate(.cgPoint, &position)!
                    let posResult = AXUIElementSetAttributeValue(
                        axWindow, kAXPositionAttribute as CFString, newPositionValue)
                    log("📍 Set position result: \(posResult.rawValue) (\(axErrorString(posResult)))")

                    // Set size
                    var size = CGSize(width: frame.width, height: frame.height)
                    let newSizeValue = AXValueCreate(.cgSize, &size)!
                    let sizeResult = AXUIElementSetAttributeValue(
                        axWindow, kAXSizeAttribute as CFString, newSizeValue)
                    log("📏 Set size result: \(sizeResult.rawValue) (\(axErrorString(sizeResult)))")

                    return posResult == .success && sizeResult == .success
                }
            } else {
                log("⚠️ Window \(index): _AXUIElementGetWindow failed")
            }
        }

        log("❌ No matching window found in app")
        return false
    }

    private func getAppForWindow(_ windowID: CGWindowID) -> NSRunningApplication? {
        let windowList =
            CGWindowListCopyWindowInfo(.optionIncludingWindow, windowID) as? [[String: Any]]
        guard let dict = windowList?.first,
            let pid = dict[kCGWindowOwnerPID as String] as? pid_t
        else {
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
                    AXUIElementCopyAttributeValue(
                        axWindow, kAXTitleAttribute as CFString, &titleRef)

                    if let title = titleRef as? String, !title.isEmpty {
                        return title
                    }

                    // Also try AXDescription as fallback
                    var descRef: CFTypeRef?
                    AXUIElementCopyAttributeValue(
                        axWindow, kAXDescriptionAttribute as CFString, &descRef)

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
        // Check current status without prompting to register app in TCC
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
        ]
        let trusted = AXIsProcessTrustedWithOptions(options as CFDictionary)

        if !trusted {
            let alert = NSAlert()
            alert.messageText = "Accessibility Permission Required"
            alert.informativeText =
                "Symposium needs accessibility permission to manage windows. Please enable it in System Settings and restart the app."
            alert.addButton(withTitle: "Open System Settings")
            alert.addButton(withTitle: "Cancel")

            if alert.runModal() == .alertFirstButtonReturn {
                // Direct user to accessibility panel
                if let url = URL(
                    string:
                        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
                ) {
                    NSWorkspace.shared.open(url)
                }
            }
        }
    }

    // MARK: - Movement Observer Setup
    
    private func setupMovementObserver() {
        guard hasAccessibilityPermission else {
            log("⚠️ Cannot setup movement observer without accessibility permission")
            return
        }
        
        let callback: AXObserverCallback = { observer, element, notification, refcon in
            guard let refcon = refcon else { return }
            let windowManager = Unmanaged<WindowManager>.fromOpaque(refcon).takeUnretainedValue()
            windowManager.handleWindowMovement(element: element, notification: notification)
        }
        
        let selfPtr = Unmanaged.passUnretained(self).toOpaque()
        let result = AXObserverCreate(getpid(), callback, &axObserver)
        
        if result == .success, let observer = axObserver {
            CFRunLoopAddSource(
                CFRunLoopGetCurrent(),
                AXObserverGetRunLoopSource(observer),
                CFRunLoopMode.defaultMode
            )
            log("✅ Movement observer setup successful")
        } else {
            log("❌ Failed to create movement observer: \(axErrorString(result))")
        }
    }
    
    private func handleWindowMovement(element: AXUIElement, notification: CFString) {
        guard notification as String == kAXMovedNotification as String else { return }
        
        log("🔄 Leader window movement detected")
        
        // Get the new position of the leader window
        guard let currentLeader = currentLeaderWindow,
              let windowID = getWindowID(from: element),
              windowID == currentLeader.id else {
            log("⚠️ Movement notification for non-leader window, ignoring")
            return
        }
        
        guard let newLeaderFrame = getWindowFrame(currentLeader.id) else {
            log("❌ Could not get new frame for leader window")
            return
        }
        
        log("🎨 Leader moved to: \(newLeaderFrame)")
        
        // Update all follower positions
        updateFollowerPositions(leaderFrame: newLeaderFrame)
        
        // Update the stored leader frame
        if let leaderIndex = stackedWindows.firstIndex(where: { $0.id == currentLeader.id }) {
            stackedWindows[leaderIndex].originalFrame = newLeaderFrame
        }
    }
    
    private func updateFollowerPositions(leaderFrame: CGRect) {
        let followers = stackedWindows.filter { !$0.isLeader }
        log("📎 Updating \(followers.count) follower positions")
        
        for follower in followers {
            let followerFrame = calculateFollowerFrame(leaderFrame: leaderFrame)
            let success = setWindowPosition(follower.id, frame: followerFrame)
            
            if success {
                log("✅ Updated follower \(follower.appName) position")
                
                // Update stored frame in the array
                if let index = stackedWindows.firstIndex(where: { $0.id == follower.id }) {
                    stackedWindows[index].originalFrame = followerFrame
                }
            } else {
                log("❌ Failed to update follower \(follower.appName) position")
            }
        }
    }
    
    private func subscribeToLeaderMovement(_ leader: WindowInfo) {
        guard let observer = axObserver else {
            log("❌ Cannot subscribe to movement - no observer")
            return
        }
        
        // Get AX element for the leader window
        guard let app = getAppForWindow(leader.id) else {
            log("❌ Cannot find app for leader window")
            return
        }
        
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        let windowsResult = AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        guard windowsResult == .success,
              let windowArray = windows as? [AXUIElement] else {
            log("❌ Failed to get windows for notification subscription")
            return
        }
        
        // Find the matching AX element
        for axWindow in windowArray {
            if let axWindowID = getWindowID(from: axWindow), axWindowID == leader.id {
                let selfPtr = Unmanaged.passUnretained(self).toOpaque()
                let result = AXObserverAddNotification(observer, axWindow, kAXMovedNotification as CFString, selfPtr)
                
                if result == .success {
                    log("✅ Subscribed to movement notifications for \(leader.appName)")
                } else {
                    log("❌ Failed to subscribe to movement notifications: \(axErrorString(result))")
                }
                return
            }
        }
        
        log("❌ Could not find AX element for leader window")
    }
    
    private func unsubscribeFromLeaderMovement(_ leader: WindowInfo) {
        guard let observer = axObserver else { return }
        
        guard let app = getAppForWindow(leader.id) else { return }
        
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var windows: CFTypeRef?
        let windowsResult = AXUIElementCopyAttributeValue(axApp, kAXWindowsAttribute as CFString, &windows)
        
        guard windowsResult == .success,
              let windowArray = windows as? [AXUIElement] else {
            return
        }
        
        for axWindow in windowArray {
            if let axWindowID = getWindowID(from: axWindow), axWindowID == leader.id {
                AXObserverRemoveNotification(observer, axWindow, kAXMovedNotification as CFString)
                log("🧹 Unsubscribed from movement notifications for \(leader.appName)")
                return
            }
        }
    }

    private func cleanupObserver() {
        if let observer = axObserver {
            CFRunLoopRemoveSource(
                CFRunLoopGetCurrent(),
                AXObserverGetRunLoopSource(observer),
                CFRunLoopMode.defaultMode
            )
        }
        axObserver = nil
        log("🧹 Movement observer cleaned up")
    }

    // MARK: - Leader Management
    
    private func switchToLeader(_ newLeader: WindowInfo) {
        guard let newLeaderIndex = stackedWindows.firstIndex(where: { $0.id == newLeader.id }) else {
            log("❌ Cannot switch to leader - window not found in stack")
            return
        }
        
        log("🔄 Switching leader to: \(newLeader.displayName)")
        
        // Update leader status in the array
        for i in 0..<stackedWindows.count {
            stackedWindows[i].isLeader = (i == newLeaderIndex)
        }
        
        // Unsubscribe from old leader's movements
        if let oldLeader = currentLeaderWindow, oldLeader.id != newLeader.id {
            unsubscribeFromLeaderMovement(oldLeader)
        }
        
        currentLeaderWindow = newLeader
        currentStackIndex = newLeaderIndex
        
        // Subscribe to new leader's movements
        subscribeToLeaderMovement(newLeader)
        
        // Move new leader to leader position and raise it
        if let leaderFrame = newLeader.originalFrame {
            let success = setWindowPosition(newLeader.id, frame: leaderFrame)
            if success {
                focusWindow(newLeader.id)
                log("👑 Leadership switched to \(newLeader.appName)")
            }
        }
    }

    // MARK: - Window Positioning
    
    private func calculateFollowerFrame(leaderFrame: CGRect) -> CGRect {
        let horizontalInset = max(minimumInset,
                                 min(maximumInset, leaderFrame.width * CGFloat(insetPercentage)))
        let verticalInset = max(minimumInset,
                               min(maximumInset, leaderFrame.height * CGFloat(insetPercentage)))
        
        return CGRect(
            x: leaderFrame.origin.x + horizontalInset,
            y: leaderFrame.origin.y + verticalInset,
            width: leaderFrame.width - (2 * horizontalInset),
            height: leaderFrame.height - (2 * verticalInset)
        )
    }

    // MARK: - Helper Functions

    private func log(_ message: String) {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        let timestamp = formatter.string(from: Date())
        let logEntry = "[\(timestamp)] \(message)\n"
        DispatchQueue.main.async {
            self.debugLog += logEntry
        }
        print(message)  // Also keep console output
        NSLog("SYMPOSIUM: %@", message)
    }

    func clearLog() {
        debugLog = ""
    }

    private func axErrorString(_ error: AXError) -> String {
        switch error {
        case .success: return "success"
        case .failure: return "failure"
        case .illegalArgument: return "illegal argument"
        case .invalidUIElement: return "invalid UI element"
        case .invalidUIElementObserver: return "invalid UI element observer"
        case .cannotComplete: return "cannot complete"
        case .attributeUnsupported: return "attribute unsupported"
        case .actionUnsupported: return "action unsupported"
        case .notificationUnsupported: return "notification unsupported"
        case .notImplemented: return "not implemented"
        case .notificationAlreadyRegistered: return "notification already registered"
        case .notificationNotRegistered: return "notification not registered"
        case .apiDisabled: return "API disabled"
        case .noValue: return "no value"
        case .parameterizedAttributeUnsupported: return "parameterized attribute unsupported"
        case .notEnoughPrecision: return "not enough precision"
        @unknown default: return "unknown error (\(error.rawValue))"
        }
    }
}
