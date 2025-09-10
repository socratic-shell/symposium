import Foundation
import ApplicationServices
import AppKit

/// Tracks window movement for stacked windows using AeroSpace-inspired approach
class WindowStackTracker {
    private var eventTap: CFMachPort?
    private var trackingTimer: Timer?
    private var leaderWindowID: CGWindowID?
    private var followerWindowIDs: [CGWindowID] = []
    private var lastLeaderPosition: CGPoint?
    
    init() {
        setupEventTap()
    }
    
    deinit {
        stopTracking()
        if let eventTap = eventTap {
            CFMachPortInvalidate(eventTap)
        }
    }
    
    /// Start tracking a leader window with its followers
    func startTracking(leaderWindowID: CGWindowID, followerWindowIDs: [CGWindowID]) {
        self.leaderWindowID = leaderWindowID
        self.followerWindowIDs = followerWindowIDs
        self.lastLeaderPosition = getWindowPosition(windowID: leaderWindowID)
        
        Logger.shared.log("WindowStackTracker: Started tracking leader \(leaderWindowID) with \(followerWindowIDs.count) followers")
    }
    
    /// Stop tracking all windows
    func stopTracking() {
        trackingTimer?.invalidate()
        trackingTimer = nil
        leaderWindowID = nil
        followerWindowIDs.removeAll()
        lastLeaderPosition = nil
        
        Logger.shared.log("WindowStackTracker: Stopped tracking")
    }
    
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
        guard let leaderWindowID = leaderWindowID else { return }
        
        switch type {
        case .leftMouseDown:
            // Check if click is on leader window
            let clickedWindowID = getWindowAtPoint(event.location)
            if clickedWindowID == leaderWindowID {
                startDragTracking()
            }
            
        case .leftMouseUp:
            stopDragTracking()
            
        default:
            break
        }
    }
    
    private func startDragTracking() {
        guard trackingTimer == nil else { return }
        
        Logger.shared.log("WindowStackTracker: Started drag tracking")
        
        trackingTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            self?.updateFollowerPositions()
        }
    }
    
    private func stopDragTracking() {
        trackingTimer?.invalidate()
        trackingTimer = nil
        
        Logger.shared.log("WindowStackTracker: Stopped drag tracking")
    }
    
    private func updateFollowerPositions() {
        guard let leaderWindowID = leaderWindowID,
              let currentPosition = getWindowPosition(windowID: leaderWindowID),
              let lastPosition = lastLeaderPosition else { return }
        
        let delta = CGPoint(x: currentPosition.x - lastPosition.x, y: currentPosition.y - lastPosition.y)
        
        // Only update if there's actual movement
        guard abs(delta.x) > 1 || abs(delta.y) > 1 else { return }
        
        // Move all follower windows by the same delta
        for followerID in followerWindowIDs {
            moveWindow(windowID: followerID, by: delta)
        }
        
        lastLeaderPosition = currentPosition
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
    
    private func moveWindow(windowID: CGWindowID, by delta: CGPoint) {
        // Get window info to find the owning process
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return
        }
        
        guard let windowInfo = windowList.first(where: { window in
            if let id = window[kCGWindowNumber as String] as? CGWindowID {
                return id == windowID
            }
            return false
        }) else { return }
        
        guard let processID = windowInfo[kCGWindowOwnerPID as String] as? pid_t else { return }
        
        let app = AXUIElementCreateApplication(processID)
        
        var windowsRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsRef)
        
        guard result == .success,
              let windows = windowsRef as? [AXUIElement] else {
            return
        }
        
        // Find the window with matching CGWindowID
        for window in windows {
            if let axWindowID = getWindowID(from: window), axWindowID == windowID {
                // Get current position
                var positionRef: CFTypeRef?
                guard AXUIElementCopyAttributeValue(window, kAXPositionAttribute as CFString, &positionRef) == .success,
                      let positionValue = positionRef else { continue }
                
                var currentPos = CGPoint.zero
                AXValueGetValue(positionValue as! AXValue, .cgPoint, &currentPos)
                
                // Calculate new position
                var newPos = CGPoint(x: currentPos.x + delta.x, y: currentPos.y + delta.y)
                let newPosValue = AXValueCreate(.cgPoint, &newPos)!
                
                // Set new position
                AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, newPosValue)
                break
            }
        }
    }
}
