import Foundation
import AVFoundation
import ApplicationServices
import AppKit

class PermissionManager: ObservableObject {
    @Published var hasAccessibilityPermission = false
    @Published var hasScreenRecordingPermission = false
    
    init() {
        checkAllPermissions()
    }
    
    func checkAllPermissions() {
        checkAccessibilityPermission()
        checkScreenRecordingPermission()
    }
    
    func checkAccessibilityPermission() {
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false
        ]
        hasAccessibilityPermission = AXIsProcessTrustedWithOptions(options as CFDictionary)
    }
    
    func requestAccessibilityPermission() {
        let options: [String: Any] = [
            kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true
        ]
        _ = AXIsProcessTrustedWithOptions(options as CFDictionary)
    }
    
    func checkScreenRecordingPermission() {
        // For macOS 10.15+, try to capture a small area to test permission
        if #available(macOS 10.15, *) {
            let displayID = CGMainDisplayID()
            if let _ = CGDisplayCreateImage(displayID) {
                hasScreenRecordingPermission = true
            } else {
                hasScreenRecordingPermission = false
            }
        } else {
            // For older macOS versions, assume permission is granted
            hasScreenRecordingPermission = true
        }
    }
    
    func requestScreenRecordingPermission() {
        // On macOS 10.15+, attempting to capture will trigger permission dialog
        if #available(macOS 10.15, *) {
            let displayID = CGMainDisplayID()
            _ = CGDisplayCreateImage(displayID)
        }
    }
    
    func openSystemPreferences(for permission: PermissionType) {
        switch permission {
        case .accessibility:
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
                NSWorkspace.shared.open(url)
            }
        case .screenRecording:
            if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture") {
                NSWorkspace.shared.open(url)
            }
        }
    }
}

enum PermissionType {
    case accessibility
    case screenRecording
}
