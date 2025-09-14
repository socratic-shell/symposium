import Foundation
import ApplicationServices
import AppKit

/// Tracks window movement and resizing for stacked windows using AeroSpace-inspired approach
class WindowStackTracker {
    private var eventTap: CFMachPort?
    private var trackingTimer: Timer?
    private var trackedWindowIDs: [CGWindowID] = []
    private var activeWindowID: CGWindowID?
    private var lastActivePosition: CGPoint?
    private var lastActiveSize: CGSize?
    
    init() {
        setupEventTap()
    }
    
    deinit {
        stopTracking()
        if let eventTap = eventTap {
            CFMachPortInvalidate(eventTap)
        }
    }
    
    /// Start tracking all windows in the stack
    func startTracking(windowIDs: [CGWindowID]) {
        self.trackedWindowIDs = windowIDs
        self.activeWindowID = nil
        self.lastActivePosition = nil
        self.lastActiveSize = nil
        
        Logger.shared.log("WindowStackTracker: Started tracking \(windowIDs.count) windows in stack")
    }
    
    /// Stop tracking all windows
    func stopTracking() {
        trackingTimer?.invalidate()
        trackingTimer = nil
        trackedWindowIDs.removeAll()
        activeWindowID = nil
        lastActivePosition = nil
        lastActiveSize = nil
        
        Logger.shared.log("WindowStackTracker: Stopped tracking")
    }
    
    /// Sets up a system-wide event tap to detect mouse clicks and drags
    /// This allows us to detect when the user starts dragging or resizing any tracked window
    /// without relying on unreliable AXObserver notifications
    private func setupEventTap() {
        let eventMask: CGEventMask = (1 << CGEventType.leftMouseDown.rawValue) |
                                   (1 << CGEventType.leftMouseDragged.rawValue) |
                                   (1 << CGEventType.leftMouseUp.rawValue)
        
        eventTap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap,
            eventsOfInterest: eventMask,
            callback: { (proxy, type, event, refcon) -> Unmanaged<CGEvent>? in
                let tracker = Unmanaged<WindowStackTracker>.fromOpaque(refcon!).takeUnretainedValue()
                tracker.handleMouseEvent(type: type, event: event)
                return Unmanaged.passUnretained(event)
            },
            userInfo: Unmanaged.passUnretained(self).toOpaque()
        )
        
        if let eventTap = eventTap {
            let runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
            CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
            CGEvent.tapEnable(tap: eventTap, enable: true)
        }
    }
    
    private func handleMouseEvent(type: CGEventType, event: CGEvent) {
        guard !trackedWindowIDs.isEmpty else { return }
        
        switch type {
        case .leftMouseDown:
            // Check if click is on any tracked window
            let clickedWindowID = getWindowAtPoint(event.location)
            if let windowID = clickedWindowID, trackedWindowIDs.contains(windowID) {
                startTracking(activeWindow: windowID)
            }
            
        case .leftMouseUp:
            stopActiveTracking()
            
        default:
            break
        }
    }
    
    private func startTracking(activeWindow: CGWindowID) {
        guard trackingTimer == nil else { return }
        
        self.activeWindowID = activeWindow
        self.lastActivePosition = getWindowPosition(windowID: activeWindow)
        self.lastActiveSize = getWindowSize(windowID: activeWindow)
        
        Logger.shared.log("WindowStackTracker: Started tracking active window \(activeWindow)")
        
        trackingTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.updateOtherWindows()
        }
    }
    
    private func stopActiveTracking() {
        trackingTimer?.invalidate()
        trackingTimer = nil
        activeWindowID = nil
        lastActivePosition = nil
        lastActiveSize = nil
        
        Logger.shared.log("WindowStackTracker: Stopped active tracking")
    }
    
    private func updateOtherWindows() {
        guard let activeWindowID = activeWindowID,
              let currentPosition = getWindowPosition(windowID: activeWindowID),
              let currentSize = getWindowSize(windowID: activeWindowID),
              let lastPosition = lastActivePosition,
              let lastSize = lastActiveSize else { return }
        
        let positionDelta = CGPoint(x: currentPosition.x - lastPosition.x, y: currentPosition.y - lastPosition.y)
        let sizeDelta = CGSize(width: currentSize.width - lastSize.width, height: currentSize.height - lastSize.height)
        
        // Only update if there's actual change
        let hasMovement = abs(positionDelta.x) > 1 || abs(positionDelta.y) > 1
        let hasResize = abs(sizeDelta.width) > 1 || abs(sizeDelta.height) > 1
        
        guard hasMovement || hasResize else { return }
        
        // Update all other windows in the stack
        let otherWindowIDs = trackedWindowIDs.filter { $0 != activeWindowID }
        for windowID in otherWindowIDs {
            if hasMovement && hasResize {
                moveAndResizeWindow(windowID: windowID, positionDelta: positionDelta, newSize: currentSize)
            } else if hasMovement {
                moveWindow(windowID: windowID, by: positionDelta)
            } else if hasResize {
                resizeWindow(windowID: windowID, to: currentSize)
            }
        }
        
        lastActivePosition = currentPosition
        lastActiveSize = currentSize
    }
    
    private func getWindowAtPoint(_ point: CGPoint) -> CGWindowID? {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements, .optionOnScreenOnly)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        
        for window in windowList {
            guard let windowID = window[kCGWindowNumber as String] as? CGWindowID,
                  let boundsDict = window[kCGWindowBounds as String] as? [String: Any],
                  let x = boundsDict["X"] as? CGFloat,
                  let y = boundsDict["Y"] as? CGFloat,
                  let width = boundsDict["Width"] as? CGFloat,
                  let height = boundsDict["Height"] as? CGFloat else { continue }
            
            let bounds = CGRect(x: x, y: y, width: width, height: height)
            if bounds.contains(point) {
                return windowID
            }
        }
        
        return nil
    }
    
    private func getWindowPosition(windowID: CGWindowID) -> CGPoint? {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        
        guard let windowInfo = windowList.first(where: { window in
            if let id = window[kCGWindowNumber as String] as? CGWindowID {
                return id == windowID
            }
            return false
        }) else { return nil }
        
        guard let boundsDict = windowInfo[kCGWindowBounds as String] as? [String: Any],
              let x = boundsDict["X"] as? CGFloat,
              let y = boundsDict["Y"] as? CGFloat else {
            return nil
        }
        
        return CGPoint(x: x, y: y)
    }
    
    private func getWindowSize(windowID: CGWindowID) -> CGSize? {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        
        guard let windowInfo = windowList.first(where: { window in
            if let id = window[kCGWindowNumber as String] as? CGWindowID {
                return id == windowID
            }
            return false
        }) else { return nil }
        
        guard let boundsDict = windowInfo[kCGWindowBounds as String] as? [String: Any],
              let width = boundsDict["Width"] as? CGFloat,
              let height = boundsDict["Height"] as? CGFloat else {
            return nil
        }
        
        return CGSize(width: width, height: height)
    }
    
    private func getWindowElement(for windowID: CGWindowID) -> AXUIElement? {
        // Get window info to find the owning process
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        
        guard let windowInfo = windowList.first(where: { window in
            if let id = window[kCGWindowNumber as String] as? CGWindowID {
                return id == windowID
            }
            return false
        }) else { return nil }
        
        guard let processID = windowInfo[kCGWindowOwnerPID as String] as? pid_t else { return nil }
        
        let app = AXUIElementCreateApplication(processID)
        
        var windowsRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsRef)
        
        guard result == .success,
              let windows = windowsRef as? [AXUIElement] else {
            return nil
        }
        
        // Find the window with matching CGWindowID
        for window in windows {
            if let axWindowID = getWindowID(from: window), axWindowID == windowID {
                return window
            }
        }
        
        return nil
    }
    
    private func resizeWindow(windowID: CGWindowID, to newSize: CGSize) {
        guard let windowElement = getWindowElement(for: windowID) else { return }
        
        var sizeValue = newSize
        let axSizeValue = AXValueCreate(.cgSize, &sizeValue)!
        AXUIElementSetAttributeValue(windowElement, kAXSizeAttribute as CFString, axSizeValue)
    }
    
    private func moveWindow(windowID: CGWindowID, by delta: CGPoint) {
        guard let windowElement = getWindowElement(for: windowID) else { return }
        
        // Get current position
        var positionRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(windowElement, kAXPositionAttribute as CFString, &positionRef) == .success,
              let positionValue = positionRef else { return }
        
        var currentPos = CGPoint.zero
        AXValueGetValue(positionValue as! AXValue, .cgPoint, &currentPos)
        
        // Calculate new position
        var newPos = CGPoint(x: currentPos.x + delta.x, y: currentPos.y + delta.y)
        let newPosValue = AXValueCreate(.cgPoint, &newPos)!
        
        // Set new position
        AXUIElementSetAttributeValue(windowElement, kAXPositionAttribute as CFString, newPosValue)
    }
    
    private func moveAndResizeWindow(windowID: CGWindowID, positionDelta: CGPoint, newSize: CGSize) {
        guard let windowElement = getWindowElement(for: windowID) else { return }
        
        // Get current position
        var positionRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(windowElement, kAXPositionAttribute as CFString, &positionRef) == .success,
              let positionValue = positionRef else { return }
        
        var currentPos = CGPoint.zero
        AXValueGetValue(positionValue as! AXValue, .cgPoint, &currentPos)
        
        // Calculate new position
        var newPos = CGPoint(x: currentPos.x + positionDelta.x, y: currentPos.y + positionDelta.y)
        let newPosValue = AXValueCreate(.cgPoint, &newPos)!
        
        // Set new position and size
        AXUIElementSetAttributeValue(windowElement, kAXPositionAttribute as CFString, newPosValue)
        
        var sizeValue = newSize
        let axSizeValue = AXValueCreate(.cgSize, &sizeValue)!
        AXUIElementSetAttributeValue(windowElement, kAXSizeAttribute as CFString, axSizeValue)
    }
}
