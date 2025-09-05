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
        
        // Set up dock icon click handling
        setupDockClickHandling()
    }
    
    // MARK: - Dock Click Handling
    
    private func setupDockClickHandling() {
        // Method 1: Override applicationShouldHandleReopen for dock click detection
        // This gets called when user clicks dock icon while app is already running
    }
    
    /// Handle dock icon clicks when app is already running
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        Logger.shared.log("AppDelegate: Dock icon clicked (hasVisibleWindows: \(flag))")
        
        // Only show panel if we have an active project
        if hasActiveProject, let projectManager = currentProjectManager {
            Logger.shared.log("AppDelegate: Showing dock panel for active project")
            
            // Calculate approximate dock click position
            // For MVP, we'll use a simple heuristic
            let dockClickPoint = estimateDockClickPosition()
            
            // Toggle panel visibility
            dockPanelManager.togglePanel(with: projectManager, near: dockClickPoint)
        } else {
            Logger.shared.log("AppDelegate: No active project, showing splash window instead")
            
            // Fallback to showing splash window if no active project
            showSplashWindow()
        }
        
        // Return false to prevent default behavior (showing all windows)
        return false
    }
    
    /// Estimate dock click position based on dock location and screen size
    private func estimateDockClickPosition() -> NSPoint {
        guard let screen = NSScreen.main else {
            return NSPoint(x: 100, y: 100)
        }
        
        let screenFrame = screen.visibleFrame
        
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
        // Find existing splash window or create new one
        if let splashWindow = NSApp.windows.first(where: { $0.title == "Symposium" }) {
            splashWindow.makeKeyAndOrderFront(nil)
        } else {
            // Open splash window via environment
            // Note: This is a temporary approach for Phase 10.1
            Logger.shared.log("AppDelegate: TODO - Implement splash window opening from AppDelegate")
        }
    }
    
    // MARK: - Project Manager Integration
    
    /// Update current project manager (called from SplashView or other coordinators)
    func setCurrentProjectManager(_ projectManager: ProjectManager?) {
        self.currentProjectManager = projectManager
        
        if let projectManager = projectManager, let project = projectManager.currentProject {
            Logger.shared.log("AppDelegate: Set active project: \(project.name)")
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
        guard let projectManager = currentProjectManager else {
            Logger.shared.log("AppDelegate: Cannot toggle panel - no active project")
            return
        }
        
        let dockPoint = estimateDockClickPosition()
        dockPanelManager.togglePanel(with: projectManager, near: dockPoint)
    }
}