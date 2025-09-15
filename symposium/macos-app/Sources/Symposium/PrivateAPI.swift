import Foundation
import ApplicationServices

// MARK: - AX Private API

// Private API for getting CGWindowID from AXUIElement
// This is the same approach used successfully by AeroSpace
@_silgen_name("_AXUIElementGetWindow")
func _AXUIElementGetWindow(_ element: AXUIElement, _ identifier: UnsafeMutablePointer<UInt32>) -> AXError

// Wrapper function for safe usage
func getWindowID(from axElement: AXUIElement) -> CGWindowID? {
    var windowID: UInt32 = 0
    let result = _AXUIElementGetWindow(axElement, &windowID)
    
    if result == .success {
        return CGWindowID(windowID)
    }
    
    return nil
}

// MARK: - CGS Private API Declarations

// Core Graphics Services (SkyLight) types
typealias CGSConnection = UInt32
typealias CGSWindowID = UInt32
typealias CGSWindowLevel = Int32

// Window ordering modes
enum CGSWindowOrderingMode: Int32 {
    case above = 1
    case below = -1
    case out = 0
}

// Common window levels
enum CGSWindowLevels {
    static let backstopMenu: CGSWindowLevel = -20
    static let normal: CGSWindowLevel = 0
    static let floating: CGSWindowLevel = 3
    static let modalPanel: CGSWindowLevel = 8
    static let utility: CGSWindowLevel = 19
    static let dock: CGSWindowLevel = 20
    static let mainMenu: CGSWindowLevel = 24
    static let status: CGSWindowLevel = 25
    static let popUpMenu: CGSWindowLevel = 101
    static let overlay: CGSWindowLevel = 102
    static let help: CGSWindowLevel = 200
    static let dragging: CGSWindowLevel = 500
    static let screenSaver: CGSWindowLevel = 1000
    static let assistiveTechHigh: CGSWindowLevel = 1500
    static let cursor: CGSWindowLevel = 2147483630
    static let maximum: CGSWindowLevel = 2147483631
}

// CGS API function declarations
@_silgen_name("CGSMainConnectionID")
func CGSMainConnectionID() -> CGSConnection

@_silgen_name("CGSOrderWindow")
func CGSOrderWindow(
    _ connection: CGSConnection,
    _ windowID: CGSWindowID,
    _ ordering: CGSWindowOrderingMode,
    _ relativeToWindow: CGSWindowID
) -> OSStatus

@_silgen_name("CGSSetWindowLevel")
func CGSSetWindowLevel(
    _ connection: CGSConnection,
    _ windowID: CGSWindowID,
    _ level: CGSWindowLevel
) -> OSStatus

@_silgen_name("CGSGetWindowLevel")
func CGSGetWindowLevel(
    _ connection: CGSConnection,
    _ windowID: CGSWindowID,
    _ level: UnsafeMutablePointer<CGSWindowLevel>
) -> OSStatus

@_silgen_name("CGSSetWindowAlpha")
func CGSSetWindowAlpha(
    _ connection: CGSConnection,
    _ windowID: CGSWindowID,
    _ alpha: Float
) -> OSStatus

@_silgen_name("CGSGetWindowAlpha")
func CGSGetWindowAlpha(
    _ connection: CGSConnection,
    _ windowID: CGSWindowID,
    _ alpha: UnsafeMutablePointer<Float>
) -> OSStatus

// MARK: - CGS Helper Functions

/// Get the main window server connection
func getCGSConnection() -> CGSConnection {
    return CGSMainConnectionID()
}

/// Order a window (show/hide/reorder)
func orderWindow(_ windowID: CGWindowID, mode: CGSWindowOrderingMode, relativeTo: CGWindowID = 0) -> OSStatus {
    let connection = getCGSConnection()
    return CGSOrderWindow(connection, CGSWindowID(windowID), mode, CGSWindowID(relativeTo))
}

/// Set window level (z-order layer)
func setWindowLevel(_ windowID: CGWindowID, level: CGSWindowLevel) -> OSStatus {
    let connection = getCGSConnection()
    return CGSSetWindowLevel(connection, CGSWindowID(windowID), level)
}

/// Get window level
func getWindowLevel(_ windowID: CGWindowID) -> (OSStatus, CGSWindowLevel) {
    let connection = getCGSConnection()
    var level: CGSWindowLevel = 0
    let result = CGSGetWindowLevel(connection, CGSWindowID(windowID), &level)
    return (result, level)
}

/// Set window transparency (0.0 = invisible, 1.0 = opaque)
func setWindowAlpha(_ windowID: CGWindowID, alpha: Float) -> OSStatus {
    let connection = getCGSConnection()
    return CGSSetWindowAlpha(connection, CGSWindowID(windowID), alpha)
}

/// Get window transparency
func getWindowAlpha(_ windowID: CGWindowID) -> (OSStatus, Float) {
    let connection = getCGSConnection()
    var alpha: Float = 1.0
    let result = CGSGetWindowAlpha(connection, CGSWindowID(windowID), &alpha)
    return (result, alpha)
}

/// Convert OSStatus to human-readable error string
func cgsErrorString(_ status: OSStatus) -> String {
    switch status {
    case noErr: return "success"
    case -50: return "parameter error"
    case -108: return "memory full error"
    case -25201: return "illegal argument"
    case -25202: return "invalid connection"
    case -25203: return "invalid context"
    case -25204: return "cannot complete"
    case -25205: return "not implemented"
    case -25206: return "range error"
    case -25207: return "type error"
    case -25208: return "no match"
    case -25209: return "invalid operation"
    case -25210: return "connection invalid"
    case -25211: return "window invalid"
    default: return "unknown error (\(status))"
    }
}