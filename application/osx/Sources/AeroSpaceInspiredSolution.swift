import ApplicationServices
import Foundation
import Cocoa

// MARK: - AeroSpace-Inspired Window Manager

class SymposiumWindowManager {
    
    // MARK: - Core Components
    private var windowStacks: [WindowStackID: WindowStack] = [:]
    private var activeTracking: Set<WindowStackID> = []
    private var dragTracker: DragTracker?
    
    // MARK: - Window Stack Management
    
    struct WindowStackID: Hashable {
        let value: String
        
        init(_ value: String) {
            self.value = value
        }
    }
    
    struct WindowStack {
        let id: WindowStackID
        var leader: WindowInfo
        var followers: [WindowInfo]
        
        struct WindowInfo {
            let element: AXUIElement
            let windowID: CGWindowID?
            var position: CGPoint
            var size: CGSize
            let pid: pid_t
            
            init(element: AXUIElement, pid: pid_t) {
                self.element = element
                self.pid = pid
                self.windowID = Self.getWindowID(from: element)
                self.position = Self.getPosition(from: element)
                self.size = Self.getSize(from: element)
            }
            
            private static func getWindowID(from element: AXUIElement) -> CGWindowID? {
                var windowID: CGWindowID = 0
                // Use AeroSpace's approach: minimal private API usage
                return _AXUIElementGetWindow(element, &windowID) == .success ? windowID : nil
            }
            
            private static func getPosition(from element: AXUIElement) -> CGPoint {
                var positionRef: CFTypeRef?
                guard AXUIElementCopyAttributeValue(element, kAXPositionAttribute, &positionRef) == .success,
                      let positionValue = positionRef else { return .zero }
                
                var position = CGPoint.zero
                AXValueGetValue(positionValue as! AXValue, .cgPoint, &position)
                return position
            }
            
            private static func getSize(from element: AXUIElement) -> CGSize {
                var sizeRef: CFTypeRef?
                guard AXUIElementCopyAttributeValue(element, kAXSizeAttribute, &sizeRef) == .success,
                      let sizeValue = sizeRef else { return .zero }
                
                var size = CGSize.zero
                AXValueGetValue(sizeValue as! AXValue, .cgSize, &size)
                return size
            }
        }
    }
    
    // MARK: - Drag Detection and Tracking
    
    private class DragTracker {
        private var timer: Timer?
        private let windowStack: WindowStack
        private let onPositionChange: (CGPoint) -> Void
        
        init(windowStack: WindowStack, onPositionChange: @escaping (CGPoint) -> Void) {
            self.windowStack = windowStack
            self.onPositionChange = onPositionChange
        }
        
        func startTracking() {
            // High-frequency polling during drag (20ms for smooth tracking)
            timer = Timer.scheduledTimer(withTimeInterval: 0.02, repeats: true) { [weak self] _ in
                self?.checkLeaderPosition()
            }
        }
        
        func stopTracking() {
            timer?.invalidate()
            timer = nil
        }
        
        private func checkLeaderPosition() {
            let currentPosition = WindowStack.WindowInfo.getPosition(from: windowStack.leader.element)
            
            if currentPosition != windowStack.leader.position {
                onPositionChange(currentPosition)
            }
        }
    }
    
    // MARK: - Public API
    
    func createWindowStack(leaderId: String, leaderWindow: AXUIElement, followerWindows: [AXUIElement]) {
        let stackID = WindowStackID(leaderId)
        
        // Get PIDs for each window
        guard let leaderPID = getProcessID(for: leaderWindow) else {
            print("Failed to get PID for leader window")
            return
        }
        
        let leader = WindowStack.WindowInfo(element: leaderWindow, pid: leaderPID)
        
        var followers: [WindowStack.WindowInfo] = []
        for followerWindow in followerWindows {
            guard let followerPID = getProcessID(for: followerWindow) else { continue }
            followers.append(WindowStack.WindowInfo(element: followerWindow, pid: followerPID))
        }
        
        let windowStack = WindowStack(id: stackID, leader: leader, followers: followers)
        windowStacks[stackID] = windowStack
        
        print("Created window stack '\(leaderId)' with \(followers.count) followers")
    }
    
    func startTrackingStack(_ stackId: String) {
        guard let stackID = WindowStackID(stackId),
              let windowStack = windowStacks[stackID],
              !activeTracking.contains(stackID) else { return }
        
        // Use AeroSpace-inspired command approach with drag detection
        detectDragStart(for: windowStack) { [weak self] in
            self?.startDragTracking(for: stackID)
        }
        
        activeTracking.insert(stackID)
    }
    
    func stopTrackingStack(_ stackId: String) {
        let stackID = WindowStackID(stackId)
        activeTracking.remove(stackID)
        
        if dragTracker?.windowStack.id == stackID {
            dragTracker?.stopTracking()
            dragTracker = nil
        }
    }
    
    // MARK: - Drag Detection
    
    private func detectDragStart(for windowStack: WindowStack, onDragStart: @escaping () -> Void) {
        // Monitor for mouse events on the leader window
        // This is more reliable than AXObserver notifications
        
        let eventMask: CGEventMask = (1 << CGEventType.leftMouseDown.rawValue) |
                                   (1 << CGEventType.leftMouseDragged.rawValue) |
                                   (1 << CGEventType.leftMouseUp.rawValue)
        
        let eventTap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap,
            eventsOfInterest: eventMask,
            callback: { (proxy, type, event, refcon) -> Unmanaged<CGEvent>? in
                // Handle mouse events
                if type == .leftMouseDown {
                    // Check if mouse is over leader window
                    let mouseLocation = event.location
                    if isPointInWindow(mouseLocation, window: windowStack.leader.element) {
                        onDragStart()
                    }
                }
                return Unmanaged.passRetained(event)
            },
            userInfo: nil
        )
        
        if let eventTap = eventTap {
            let runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
            CFRunLoopAddSource(CFRunLoopGetCurrent(), runLoopSource, .commonModes)
            CGEvent.tapEnable(tap: eventTap, enable: true)
        }
    }
    
    private func startDragTracking(for stackID: WindowStackID) {
        guard let windowStack = windowStacks[stackID] else { return }
        
        print("Starting drag tracking for stack '\(stackID.value)'")
        
        dragTracker = DragTracker(windowStack: windowStack) { [weak self] newPosition in
            self?.handleLeaderPositionChange(stackID: stackID, newPosition: newPosition)
        }
        
        dragTracker?.startTracking()
    }
    
    // MARK: - Window Synchronization
    
    private func handleLeaderPositionChange(stackID: WindowStackID, newPosition: CGPoint) {
        guard var windowStack = windowStacks[stackID] else { return }
        
        let deltaX = newPosition.x - windowStack.leader.position.x
        let deltaY = newPosition.y - windowStack.leader.position.y
        
        // Only sync if there's significant movement (avoid jitter)
        guard abs(deltaX) > 1.0 || abs(deltaY) > 1.0 else { return }
        
        // Update leader position
        windowStack.leader.position = newPosition
        windowStacks[stackID] = windowStack
        
        // Move all followers
        synchronizeFollowers(stackID: stackID, deltaX: deltaX, deltaY: deltaY)
    }
    
    private func synchronizeFollowers(stackID: WindowStackID, deltaX: CGFloat, deltaY: CGFloat) {
        guard var windowStack = windowStacks[stackID] else { return }
        
        for i in windowStack.followers.indices {
            let currentFollower = windowStack.followers[i]
            let newPosition = CGPoint(
                x: currentFollower.position.x + deltaX,
                y: currentFollower.position.y + deltaY
            )
            
            // Move the follower window
            if moveWindow(currentFollower.element, to: newPosition) {
                // Update stored position
                windowStack.followers[i].position = newPosition
            }
        }
        
        windowStacks[stackID] = windowStack
    }
    
    // MARK: - Window Manipulation
    
    @discardableResult
    private func moveWindow(_ window: AXUIElement, to position: CGPoint) -> Bool {
        let positionValue = AXValueCreate(.cgPoint, &position)!
        let result = AXUIElementSetAttributeValue(window, kAXPositionAttribute, positionValue)
        
        if result != .success {
            print("Failed to move window: \(result)")
            return false
        }
        
        return true
    }
    
    // MARK: - Helper Functions
    
    private func getProcessID(for window: AXUIElement) -> pid_t? {
        var pid: pid_t = 0
        let result = AXUIElementGetPid(window, &pid)
        return result == .success ? pid : nil
    }
    
    private func isPointInWindow(_ point: CGPoint, window: AXUIElement) -> Bool {
        let position = WindowStack.WindowInfo.getPosition(from: window)
        let size = WindowStack.WindowInfo.getSize(from: window)
        
        let windowRect = CGRect(origin: position, size: size)
        return windowRect.contains(point)
    }
}

// MARK: - Private API Declaration
// Following AeroSpace's minimal private API approach
extern "C" AXError _AXUIElementGetWindow(AXUIElementRef element, CGWindowID* windowID)

// MARK: - Usage Example

class SymposiumApp {
    private let windowManager = SymposiumWindowManager()
    
    func setupWindowStacking() {
        // Example: Stack VS Code with Terminal
        guard let frontmostApp = NSWorkspace.shared.frontmostApplication else { return }
        
        let appElement = AXUIElementCreateApplication(frontmostApp.processIdentifier)
        
        // Get windows from the application
        var windowsRef: CFTypeRef?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute, &windowsRef) == .success,
              let windows = windowsRef as? [AXUIElement],
              windows.count >= 2 else {
            print("Need at least 2 windows to create a stack")
            return
        }
        
        let leaderWindow = windows[0]
        let followerWindows = Array(windows[1...])
        
        // Create the window stack
        windowManager.createWindowStack(
            leaderId: "main-stack",
            leaderWindow: leaderWindow,
            followerWindows: followerWindows
        )
        
        // Start tracking
        windowManager.startTrackingStack("main-stack")
        
        print("Window stacking activated!")
    }
    
    func tearDown() {
        windowManager.stopTrackingStack("main-stack")
    }
}

// MARK: - SwiftUI Integration Example

import SwiftUI

struct SymposiumControlPanel: View {
    @StateObject private var windowManager = SymposiumWindowManager()
    @State private var isTracking = false
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Symposium Window Manager")
                .font(.title)
            
            Button(action: {
                if isTracking {
                    windowManager.stopTrackingStack("main-stack")
                } else {
                    setupAndStartTracking()
                }
                isTracking.toggle()
            }) {
                Text(isTracking ? "Stop Tracking" : "Start Window Stacking")
                    .foregroundColor(.white)
                    .padding()
                    .background(isTracking ? Color.red : Color.blue)
                    .cornerRadius(8)
            }
            
            Text("Status: \(isTracking ? "Active" : "Inactive")")
                .foregroundColor(isTracking ? .green : .gray)
        }
        .padding()
    }
    
    private func setupAndStartTracking() {
        // Setup logic similar to SymposiumApp example
    }
}
