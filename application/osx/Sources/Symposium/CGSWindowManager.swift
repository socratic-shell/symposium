import Foundation
import SwiftUI
import AppKit

// MARK: - CGS Window Manager for Testing
//
// NOTE: This code is from the original window management testing interface.
// It's not currently used by the new project management UI (Phase 1),
// but will be integrated in Phase 4 for taskspace window tiling and management.

class CGSWindowManager: ObservableObject {
    
    // MARK: - Test Window Info
    
    struct TestWindowInfo: Identifiable {
        let id: CGWindowID
        let title: String
        let appName: String
        let originalFrame: CGRect
        var currentLevel: CGSWindowLevel = 0
        var currentAlpha: Float = 1.0
        var isOrderedOut: Bool = false
        
        var displayName: String {
            let prefix = appName == "Symposium" ? "🧪 " : ""
            if !title.isEmpty {
                return "\(prefix)\(appName): \(title)"
            } else {
                return "\(prefix)\(appName): Window \(id)"
            }
        }
        
        var isOwnWindow: Bool {
            return appName == "Symposium"
        }
    }
    
    // MARK: - Published Properties
    
    @Published var allWindows: [TestWindowInfo] = []
    @Published var selectedWindow: TestWindowInfo?
    @Published var testLog: String = ""
    @Published var hasAccessibilityPermission: Bool = false
    
    // MARK: - Original State Tracking
    
    private var originalStates: [CGWindowID: (level: CGSWindowLevel, alpha: Float)] = [:]
    
    // MARK: - Initialization
    
    init() {
        checkAccessibilityPermission()
        refreshWindowList()
    }
    
    deinit {
        restoreAllWindows()
    }
    
    // MARK: - Permission Management
    
    func checkAccessibilityPermission() {
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
        ]
        hasAccessibilityPermission = AXIsProcessTrustedWithOptions(options as CFDictionary)
        log("🔐 Accessibility permission: \(hasAccessibilityPermission ? "✅ Granted" : "❌ Required")")
    }
    
    func requestAccessibilityPermission() {
        let alert = NSAlert()
        alert.messageText = "Accessibility Permission Required"
        alert.informativeText = "CGS Window Testing requires accessibility permission to manipulate windows."
        alert.addButton(withTitle: "Open System Settings")
        alert.addButton(withTitle: "Cancel")
        
        if alert.runModal() == .alertFirstButtonReturn {
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                NSWorkspace.shared.open(url)
            }
        }
    }
    
    // MARK: - Window Discovery
    
    func refreshWindowList() {
        let options = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
        let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
        
        var newWindows: [TestWindowInfo] = []
        
        for dict in windowList {
            guard let windowID = dict[kCGWindowNumber as String] as? CGWindowID,
                  let appName = dict[kCGWindowOwnerName as String] as? String,
                  dict[kCGWindowLayer as String] as? Int == 0,  // Normal windows only
                  let bounds = dict[kCGWindowBounds as String] as? [String: CGFloat]
            else {
                continue
            }
            
            // Include all windows (including our own for testing)
            
            let title = dict[kCGWindowName as String] as? String ?? ""
            
            let frame = CGRect(
                x: bounds["X"] ?? 0,
                y: bounds["Y"] ?? 0,
                width: bounds["Width"] ?? 0,
                height: bounds["Height"] ?? 0
            )
            
            var windowInfo = TestWindowInfo(
                id: windowID,
                title: title,
                appName: appName,
                originalFrame: frame
            )
            
            // Get current CGS state
            updateWindowState(&windowInfo)
            
            newWindows.append(windowInfo)
        }
        
        allWindows = newWindows.sorted { $0.appName < $1.appName }
        log("🔄 Found \(allWindows.count) windows")
    }
    
    private func updateWindowState(_ windowInfo: inout TestWindowInfo) {
        // Get current window level
        let (levelStatus, level) = getWindowLevel(windowInfo.id)
        if levelStatus == noErr {
            windowInfo.currentLevel = level
        }
        
        // Get current window alpha
        let (alphaStatus, alpha) = getWindowAlpha(windowInfo.id)
        if alphaStatus == noErr {
            windowInfo.currentAlpha = alpha
        }
        
        // Store original state if not already stored
        if originalStates[windowInfo.id] == nil {
            originalStates[windowInfo.id] = (level: level, alpha: alpha)
        }
    }
    
    // MARK: - CGS Test Operations
    
    func orderWindowOut(_ windowID: CGWindowID) {
        let startTime = CFAbsoluteTimeGetCurrent()
        let result = orderWindow(windowID, mode: .out)
        let duration = CFAbsoluteTimeGetCurrent() - startTime
        
        let status = result == noErr ? "✅" : "❌"
        log("🫥 Order Out: \(status) Window \(windowID) - \(cgsErrorString(result)) (\(String(format: "%.3f", duration * 1000))ms)")
        
        if result == noErr {
            updateSelectedWindowState { $0.isOrderedOut = true }
        }
    }
    
    func orderWindowIn(_ windowID: CGWindowID) {
        let startTime = CFAbsoluteTimeGetCurrent()
        let result = orderWindow(windowID, mode: .above)
        let duration = CFAbsoluteTimeGetCurrent() - startTime
        
        let status = result == noErr ? "✅" : "❌"
        log("👁️ Order In: \(status) Window \(windowID) - \(cgsErrorString(result)) (\(String(format: "%.3f", duration * 1000))ms)")
        
        if result == noErr {
            updateSelectedWindowState { $0.isOrderedOut = false }
        }
    }
    
    func orderWindowBelow(_ windowID: CGWindowID, relativeTo: CGWindowID? = nil) {
        let startTime = CFAbsoluteTimeGetCurrent()
        let relativeWindow = relativeTo ?? 0
        let result = orderWindow(windowID, mode: .below, relativeTo: relativeWindow)
        let duration = CFAbsoluteTimeGetCurrent() - startTime
        
        let status = result == noErr ? "✅" : "❌"
        let relativeDesc = relativeTo != nil ? " (relative to \(relativeTo!))" : " (to back)"
        log("⬇️ Order Below: \(status) Window \(windowID)\(relativeDesc) - \(cgsErrorString(result)) (\(String(format: "%.3f", duration * 1000))ms)")
    }
    
    func setWindowLevel(_ windowID: CGWindowID, level: CGSWindowLevel) {
        let startTime = CFAbsoluteTimeGetCurrent()
        let result = Symposium.setWindowLevel(windowID, level: level)
        let duration = CFAbsoluteTimeGetCurrent() - startTime
        
        let status = result == noErr ? "✅" : "❌"
        log("🎚️ Set Level: \(status) Window \(windowID) to \(level) - \(cgsErrorString(result)) (\(String(format: "%.3f", duration * 1000))ms)")
        
        if result == noErr {
            updateSelectedWindowState { $0.currentLevel = level }
        }
    }
    
    func setWindowAlpha(_ windowID: CGWindowID, alpha: Float) {
        let startTime = CFAbsoluteTimeGetCurrent()
        let result = Symposium.setWindowAlpha(windowID, alpha: alpha)
        let duration = CFAbsoluteTimeGetCurrent() - startTime
        
        let status = result == noErr ? "✅" : "❌"
        let percentage = Int(alpha * 100)
        log("🌫️ Set Alpha: \(status) Window \(windowID) to \(percentage)% - \(cgsErrorString(result)) (\(String(format: "%.3f", duration * 1000))ms)")
        
        if result == noErr {
            updateSelectedWindowState { $0.currentAlpha = alpha }
        }
    }
    
    func restoreWindow(_ windowID: CGWindowID) {
        guard let originalState = originalStates[windowID] else {
            log("❌ No original state stored for window \(windowID)")
            return
        }
        
        log("↩️ Restoring window \(windowID) to original state...")
        
        // Restore level
        let levelResult = Symposium.setWindowLevel(windowID, level: originalState.level)
        let levelStatus = levelResult == noErr ? "✅" : "❌"
        log("   Level: \(levelStatus) \(originalState.level) - \(cgsErrorString(levelResult))")
        
        // Restore alpha
        let alphaResult = Symposium.setWindowAlpha(windowID, alpha: originalState.alpha)
        let alphaStatus = alphaResult == noErr ? "✅" : "❌"
        log("   Alpha: \(alphaStatus) \(Int(originalState.alpha * 100))% - \(cgsErrorString(alphaResult))")
        
        // Ensure it's ordered in
        let orderResult = orderWindow(windowID, mode: .above)
        let orderStatus = orderResult == noErr ? "✅" : "❌"
        log("   Order: \(orderStatus) Above - \(cgsErrorString(orderResult))")
        
        // Update UI state
        if let index = allWindows.firstIndex(where: { $0.id == windowID }) {
            allWindows[index].currentLevel = originalState.level
            allWindows[index].currentAlpha = originalState.alpha
            allWindows[index].isOrderedOut = false
        }
    }
    
    func restoreAllWindows() {
        log("🔄 Restoring all modified windows...")
        for (windowID, _) in originalStates {
            restoreWindow(windowID)
        }
    }
    
    // MARK: - Convenience Methods
    
    func makeWindowInvisible(_ windowID: CGWindowID) {
        log("👻 Making window \(windowID) invisible using best method...")
        // Try order out first (most efficient according to research)
        orderWindowOut(windowID)
    }
    
    func makeWindowVisible(_ windowID: CGWindowID) {
        log("👁️ Making window \(windowID) visible...")
        restoreWindow(windowID)
    }
    
    func sendWindowBehindDesktop(_ windowID: CGWindowID) {
        log("🏔️ Sending window \(windowID) behind desktop...")
        setWindowLevel(windowID, level: CGSWindowLevels.backstopMenu)
    }
    
    func makeWindowFloat(_ windowID: CGWindowID) {
        log("☁️ Making window \(windowID) float above others...")
        setWindowLevel(windowID, level: CGSWindowLevels.floating)
    }
    
    // MARK: - Helper Methods
    
    private func updateSelectedWindowState(_ update: (inout TestWindowInfo) -> Void) {
        guard var selected = selectedWindow,
              let index = allWindows.firstIndex(where: { $0.id == selected.id }) else {
            return
        }
        
        update(&selected)
        allWindows[index] = selected
        selectedWindow = selected
    }
    
    private func log(_ message: String) {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss.SSS"
        let timestamp = formatter.string(from: Date())
        let logEntry = "[\(timestamp)] \(message)\n"
        
        DispatchQueue.main.async {
            self.testLog += logEntry
        }
        
        print("CGS_TEST: \(message)")
        NSLog("CGS_TEST: %@", message)
    }
    
    func clearLog() {
        testLog = ""
    }
    
    // MARK: - Test Window Creation
    
    func createTestWindow() {
        log("🪟 Creating new test window...")
        
        // Create a new SwiftUI window
        let testWindow = NSWindow(
            contentRect: NSRect(x: 200, y: 200, width: 400, height: 300),
            styleMask: [.titled, .closable, .resizable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        
        testWindow.title = "Symposium Test Window"
        testWindow.contentView = NSHostingView(rootView: TestWindowView())
        testWindow.makeKeyAndOrderFront(nil)
        
        // Store the window to keep it alive
        testWindows.append(testWindow)
        
        // Refresh the window list to pick up our new window
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            self.refreshWindowList()
        }
    }
    
    // Keep references to test windows
    private var testWindows: [NSWindow] = []
}

// MARK: - Test Window Content View

struct TestWindowView: View {
    var body: some View {
        VStack(spacing: 20) {
            Text("🧪 CGS Test Window")
                .font(.title2)
                .fontWeight(.semibold)
            
            Text("This window is created by Symposium and should be controllable via CGS APIs.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            
            Text("Window ID: \(NSApp.keyWindow?.windowNumber ?? 0)")
                .font(.caption)
                .foregroundColor(.secondary)
            
            Spacer()
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(NSColor.controlBackgroundColor))
    }
}