import Foundation
import ApplicationServices

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