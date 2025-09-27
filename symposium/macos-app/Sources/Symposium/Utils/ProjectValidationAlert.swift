import AppKit
import SwiftUI

/// Notification name for triggering new project dialog
extension Notification.Name {
    static let showNewProjectDialog = Notification.Name("showNewProjectDialog")
}

/// Utility struct for presenting project validation error alerts consistently across the app
struct ProjectValidationAlert {
    
    /// Present an alert for a project validation error
    /// - Parameters:
    ///   - error: The ProjectValidationError to present
    ///   - window: The window to present the alert as a sheet modal
    @MainActor
    static func present(for error: ProjectValidationError, in window: NSWindow) {
        let alert = NSAlert()
        alert.messageText = error.errorDescription ?? "Project Validation Failed"
        alert.informativeText = error.recoverySuggestion ?? "Please try again with a different directory."
        alert.alertStyle = .warning
        alert.addButton(withTitle: "OK")
        alert.addButton(withTitle: "Create New Project")
        
        alert.beginSheetModal(for: window) { response in
            if response == .alertSecondButtonReturn {
                // Handle "Create New Project" action by posting notification
                NotificationCenter.default.post(
                    name: .showNewProjectDialog,
                    object: nil
                )
            }
        }
    }
    
    /// Present an alert for a project validation error using a SwiftUI view context
    /// - Parameters:
    ///   - error: The ProjectValidationError to present
    ///   - view: The SwiftUI view to find the window from
    @MainActor
    static func present(for error: ProjectValidationError, from view: NSView) {
        guard let window = view.window else {
            Logger.shared.log("ERROR: Could not find window for ProjectValidationAlert presentation")
            return
        }
        
        present(for: error, in: window)
    }
    
    /// Present an alert for a project validation error using the current key window
    /// - Parameter error: The ProjectValidationError to present
    @MainActor
    static func present(for error: ProjectValidationError) {
        guard let window = NSApplication.shared.keyWindow else {
            Logger.shared.log("ERROR: Could not find key window for ProjectValidationAlert presentation")
            return
        }
        
        present(for: error, in: window)
    }
}