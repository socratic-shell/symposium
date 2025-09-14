import AppKit
import SwiftUI
import Combine

/// App delegate for handling dock clicks and application lifecycle
class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    
    /// Current project manager (for window coordination)
    @Published var currentProjectManager: ProjectManager? {
        didSet {
            // Cancel previous subscription
            projectManagerCancellable?.cancel()
            
            // Subscribe to new project manager changes
            if let projectManager = currentProjectManager {
                projectManagerCancellable = projectManager.objectWillChange
                    .sink { [weak self] in
                        self?.objectWillChange.send()
                    }
            }
        }
    }
    
    private var projectManagerCancellable: AnyCancellable?
    
    /// Track if we have a current project loaded
    private var hasActiveProject: Bool {
        return currentProjectManager?.currentProject != nil
    }
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application finished launching")
        Logger.shared.log("AppDelegate: Current project manager: \(currentProjectManager == nil ? "nil" : "exists")")
        
        // Set up dock icon click handling
        NSApp.setActivationPolicy(.regular)
        Logger.shared.log("AppDelegate: Dock icon click handling configured")
    }
    
    /// Handle dock icon clicks
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        Logger.shared.log("AppDelegate: Dock icon clicked, hasVisibleWindows: \(flag)")
        Logger.shared.log("AppDelegate: Current project manager exists: \(currentProjectManager != nil)")
        Logger.shared.log("AppDelegate: Has active project: \(hasActiveProject)")
        
        // For now, just log - window management is handled by the new architecture
        Logger.shared.log("AppDelegate: Using window-based architecture, dock clicks handled by window system")
        
        return true
    }
    
    /// Handle application becoming active (alt-tab, app switcher)
    func applicationDidBecomeActive(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application became active (alt-tab or app switcher)")
        Logger.shared.log("AppDelegate: Current project manager exists: \(currentProjectManager != nil)")
        Logger.shared.log("AppDelegate: Has active project: \(hasActiveProject)")
        
        // No longer showing dock panel - using window-based architecture
        Logger.shared.log("AppDelegate: Using window-based architecture, no dock panel to show")
    }
    
    /// Handle application losing focus (switching to other apps)
    func applicationDidResignActive(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application resigned active (switched to other app)")
        // No dock panel to hide anymore
    }
}
