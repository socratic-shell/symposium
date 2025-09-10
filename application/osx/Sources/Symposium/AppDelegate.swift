import AppKit
import SwiftUI

extension Notification.Name {
    static let openSplashWindow = Notification.Name("openSplashWindow")
}

/// App delegate to handle dock icon clicks and other app-level events
class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    
    /// Dock panel manager for handling panel display
    private let dockPanelManager = DockPanelManager()
    
    /// Current project manager (for showing panel content)
    @Published var currentProjectManager: ProjectManager?
    
    /// Phase 22: Callback for closing the active project (set by SplashView)
    private var closeProjectCallback: (() -> Void)?
    
    /// Track if we have a current project loaded
    private var hasActiveProject: Bool {
        return currentProjectManager?.currentProject != nil
    }
    
    func applicationDidFinishLaunching(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application finished launching")
        Logger.shared.log("AppDelegate: DockPanelManager initialized")
        Logger.shared.log("AppDelegate: Current project manager: \(currentProjectManager == nil ? "nil" : "exists")")
        
        // Set up dock icon click handling
        setupDockClickHandling()
    }
    
    // MARK: - Dock Click Handling
    
    private func setupDockClickHandling() {
        Logger.shared.log("AppDelegate: Setting up dock click handling")
        // Method 1: Override applicationShouldHandleReopen for dock click detection
        // This gets called when user clicks dock icon while app is already running
    }
    
    /// Handle application becoming active (alt-tab, app switcher)
    func applicationDidBecomeActive(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application became active (alt-tab or app switcher)")
        Logger.shared.log("AppDelegate: Current project manager exists: \(currentProjectManager != nil)")
        Logger.shared.log("AppDelegate: Has active project: \(hasActiveProject)")
        
        // Show panel if we have an active project, otherwise show splash
        if hasActiveProject, let projectManager = currentProjectManager {
            Logger.shared.log("AppDelegate: Showing dock panel for active project on activation")
            
            dockPanelManager.showPanel(with: projectManager, onCloseProject: closeProjectCallback, onDismiss: hideDockPanel)
        } else {
            Logger.shared.log("AppDelegate: No active project, showing splash window on activation")
            showSplashWindow()
        }
    }
    
    /// Handle application losing focus (switching to other apps)
    func applicationDidResignActive(_ notification: Notification) {
        Logger.shared.log("AppDelegate: Application resigned active (switched to other app)")
        
        // Hide panel when user switches to another application
        hideDockPanel()
        Logger.shared.log("AppDelegate: Hidden dock panel on app resign")
    }
    
    /// Handle dock icon clicks when app is already running
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        Logger.shared.log("AppDelegate: Dock icon clicked (hasVisibleWindows: \(flag))")
        Logger.shared.log("AppDelegate: Current project manager exists: \(currentProjectManager != nil)")
        Logger.shared.log("AppDelegate: Has active project: \(hasActiveProject)")
        
        if let projectManager = currentProjectManager {
            Logger.shared.log("AppDelegate: Project manager found - project: \(projectManager.currentProject?.name ?? "nil")")
        }
        
        // Only show panel if we have an active project
        if hasActiveProject, let projectManager = currentProjectManager {
            Logger.shared.log("AppDelegate: Showing dock panel for active project: \(projectManager.currentProject?.name ?? "unknown")")
            
            // Toggle panel visibility
            Logger.shared.log("AppDelegate: Calling dockPanelManager.togglePanel")
            dockPanelManager.togglePanel(with: projectManager, onCloseProject: closeProjectCallback, onDismiss: hideDockPanel)
        } else {
            Logger.shared.log("AppDelegate: No active project, showing splash window instead")
            Logger.shared.log("AppDelegate: currentProjectManager nil: \(currentProjectManager == nil)")
            if let pm = currentProjectManager {
                Logger.shared.log("AppDelegate: currentProject nil: \(pm.currentProject == nil)")
            }
            
            // Fallback to showing splash window if no active project
            showSplashWindow()
        }
        
        // Return false to prevent default behavior (showing all windows)
        Logger.shared.log("AppDelegate: Returning false from applicationShouldHandleReopen")
        return false
    }
    
    
    /// Show splash window when no active project
    private func showSplashWindow() {
        Logger.shared.log("AppDelegate: Looking for existing splash window")
        // Find existing splash window or create new one
        if let splashWindow = NSApp.windows.first(where: { $0.title == "Symposium" }) {
            Logger.shared.log("AppDelegate: Found existing splash window, bringing to front")
            splashWindow.makeKeyAndOrderFront(nil)
        } else {
            Logger.shared.log("AppDelegate: No splash window found, posting notification to open it")
            // Post notification for SwiftUI app to handle window opening
            NotificationCenter.default.post(name: .openSplashWindow, object: nil)
        }
    }
    
    // MARK: - Project Manager Integration
    
    /// Update current project manager (called from SplashView or other coordinators)
    func setCurrentProjectManager(_ projectManager: ProjectManager?, closeCallback: (() -> Void)? = nil) {
        Logger.shared.log("AppDelegate: setCurrentProjectManager called")
        Logger.shared.log("AppDelegate: Previous project manager: \(self.currentProjectManager == nil ? "nil" : "exists")")
        Logger.shared.log("AppDelegate: New project manager: \(projectManager == nil ? "nil" : "exists")")
        Logger.shared.log("AppDelegate: Close callback provided: \(closeCallback != nil)")
        
        self.currentProjectManager = projectManager
        self.closeProjectCallback = closeCallback
        
        if let projectManager = projectManager, let project = projectManager.currentProject {
            Logger.shared.log("AppDelegate: Set active project: \(project.name)")
            Logger.shared.log("AppDelegate: Project directory: \(project.directoryPath)")
            Logger.shared.log("AppDelegate: Project taskspaces count: \(project.taskspaces.count)")
        } else if projectManager != nil {
            Logger.shared.log("AppDelegate: Set project manager but no current project loaded")
        } else {
            Logger.shared.log("AppDelegate: Cleared active project")
        }
    }
    
    /// Get shared app delegate instance
    static var shared: AppDelegate? {
        return NSApp.delegate as? AppDelegate
    }
}

/// Extension for global dock panel access
extension AppDelegate {
    
    /// Show dock panel with specific project
    func showDockPanel(with projectManager: ProjectManager) {
        dockPanelManager.showPanel(with: projectManager, onCloseProject: closeProjectCallback, onDismiss: hideDockPanel)
    }
    
    /// Hide dock panel
    func hideDockPanel() {
        dockPanelManager.hidePanel()
    }
    
    /// Toggle dock panel visibility
    func toggleDockPanel() {
        Logger.shared.log("AppDelegate: toggleDockPanel called (⌘⇧P menu item)")
        Logger.shared.log("AppDelegate: Current project manager exists: \(currentProjectManager != nil)")
        Logger.shared.log("AppDelegate: Has active project: \(hasActiveProject)")
        
        guard let projectManager = currentProjectManager else {
            Logger.shared.log("AppDelegate: Cannot toggle panel - no active project")
            Logger.shared.log("AppDelegate: currentProjectManager is nil")
            return
        }
        
        if let project = projectManager.currentProject {
            Logger.shared.log("AppDelegate: Toggling panel for project: \(project.name)")
        } else {
            Logger.shared.log("AppDelegate: ProjectManager exists but no current project")
        }
        
        Logger.shared.log("AppDelegate: Calling dockPanelManager.togglePanel from menu action")
        dockPanelManager.togglePanel(with: projectManager, onCloseProject: closeProjectCallback, onDismiss: hideDockPanel)
    }
}