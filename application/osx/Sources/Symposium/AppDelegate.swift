import AppKit
import SwiftUI

/// App delegate to handle dock icon clicks and other app-level events
class AppDelegate: NSObject, NSApplicationDelegate {
    
    /// Dock panel manager for handling panel display
    private let dockPanelManager = DockPanelManager()
    
    /// Current project manager (for showing panel content)
    private var currentProjectManager: ProjectManager?
    
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
            
            // Calculate approximate dock click position
            // For MVP, we'll use a simple heuristic
            let dockClickPoint = estimateDockClickPosition()
            Logger.shared.log("AppDelegate: Calculated dock click position: \(dockClickPoint)")
            
            // Toggle panel visibility
            Logger.shared.log("AppDelegate: Calling dockPanelManager.togglePanel")
            dockPanelManager.togglePanel(with: projectManager, near: dockClickPoint)
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
    
    /// Estimate dock click position based on dock location and screen size
    private func estimateDockClickPosition() -> NSPoint {
        guard let screen = NSScreen.main else {
            Logger.shared.log("AppDelegate: No main screen found, using fallback position")
            return NSPoint(x: 100, y: 100)
        }
        
        let screenFrame = screen.visibleFrame
        Logger.shared.log("AppDelegate: Screen frame: \(screenFrame)")
        
        // For MVP, assume dock is at bottom center
        // TODO: Implement proper dock position detection in Phase 50
        let dockClickPoint = NSPoint(
            x: screenFrame.midX,
            y: screenFrame.minY + 30  // Approximate dock height
        )
        
        Logger.shared.log("AppDelegate: Estimated dock click at: \(dockClickPoint)")
        return dockClickPoint
    }
    
    /// Show splash window when no active project
    private func showSplashWindow() {
        Logger.shared.log("AppDelegate: Looking for existing splash window")
        // Find existing splash window or create new one
        if let splashWindow = NSApp.windows.first(where: { $0.title == "Symposium" }) {
            Logger.shared.log("AppDelegate: Found existing splash window, bringing to front")
            splashWindow.makeKeyAndOrderFront(nil)
        } else {
            // Open splash window via environment
            // Note: This is a temporary approach for Phase 10.1
            Logger.shared.log("AppDelegate: No splash window found")
            Logger.shared.log("AppDelegate: TODO - Implement splash window opening from AppDelegate")
        }
    }
    
    // MARK: - Project Manager Integration
    
    /// Update current project manager (called from SplashView or other coordinators)
    func setCurrentProjectManager(_ projectManager: ProjectManager?) {
        Logger.shared.log("AppDelegate: setCurrentProjectManager called")
        Logger.shared.log("AppDelegate: Previous project manager: \(self.currentProjectManager == nil ? "nil" : "exists")")
        Logger.shared.log("AppDelegate: New project manager: \(projectManager == nil ? "nil" : "exists")")
        
        self.currentProjectManager = projectManager
        
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
    func showDockPanel(with projectManager: ProjectManager, at point: NSPoint? = nil) {
        let dockPoint = point ?? estimateDockClickPosition()
        dockPanelManager.showPanel(with: projectManager, near: dockPoint)
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
        
        let dockPoint = estimateDockClickPosition()
        Logger.shared.log("AppDelegate: Calling dockPanelManager.togglePanel from menu action")
        dockPanelManager.togglePanel(with: projectManager, near: dockPoint)
    }
}