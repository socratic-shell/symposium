import AppKit
import SwiftUI

/// Panel size constraints for dock panel layout
struct PanelConstraints {
    static let defaultWidth: CGFloat = 400
    static let minWidth: CGFloat = 300
    static let maxWidth: CGFloat = 500
    static let minHeight: CGFloat = 200
    static let maxHeight: CGFloat = 800
}

/// NSHostingView wrapper specifically designed for DockPanel SwiftUI integration
class DockPanelHostingView<Content: View>: NSHostingView<Content> {
    
    required init(rootView: Content) {
        Logger.shared.log("DockPanelHostingView: Initializing with root view")
        super.init(rootView: rootView)
        setupHostingView()
    }
    
    @MainActor required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
    
    private func setupHostingView() {
        Logger.shared.log("DockPanelHostingView: Setting up hosting view constraints")
        
        // Configure hosting view for panel layout
        self.translatesAutoresizingMaskIntoConstraints = false
        
        // Remove fixed width/height constraints to allow responsive sizing
        // The panel size will be controlled by DockPanelManager's calculations
        
        Logger.shared.log("DockPanelHostingView: Constraints configured successfully (responsive sizing enabled)")
    }
    
    /// Calculate ideal panel size based on content and screen constraints
    func calculateIdealSize(for screen: NSScreen? = nil) -> NSSize {
        let screen = screen ?? NSScreen.main
        guard let screen = screen else {
            return NSSize(width: PanelConstraints.defaultWidth, height: 600)
        }
        
        let screenFrame = screen.visibleFrame
        
        // Calculate ideal height based on screen size (leaving margins)
        let availableHeight = screenFrame.height - 100 // Leave 100pt margin
        let idealHeight = min(availableHeight, PanelConstraints.maxHeight)
        let constrainedHeight = max(PanelConstraints.minHeight, idealHeight)
        
        return NSSize(
            width: PanelConstraints.defaultWidth,
            height: constrainedHeight
        )
    }
    
    /// Update panel size while respecting constraints
    func updatePanelSize(to newSize: NSSize) {
        let constrainedSize = NSSize(
            width: max(PanelConstraints.minWidth, min(newSize.width, PanelConstraints.maxWidth)),
            height: max(PanelConstraints.minHeight, min(newSize.height, PanelConstraints.maxHeight))
        )
        
        // Update the panel window size if we're in a panel
        if let panel = self.window as? DockPanel {
            var newFrame = panel.frame
            newFrame.size = constrainedSize
            panel.setFrame(newFrame, display: true, animate: true)
        }
    }
}

/// Extension to make DockPanelHostingView work with ProjectView specifically
extension DockPanelHostingView where Content == ProjectView {
    
    /// Convenience initializer for ProjectView integration
    convenience init(projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil, onDismiss: (() -> Void)? = nil) {
        let projectView = ProjectView(projectManager: projectManager, onCloseProject: onCloseProject, onDismiss: onDismiss)
        self.init(rootView: projectView)
    }
    
    /// Update the project view with new project manager (for project switching)
    func updateProjectManager(_ projectManager: ProjectManager) {
        let newProjectView = ProjectView(projectManager: projectManager)
        self.rootView = newProjectView
    }
}

/// Helper for creating SwiftUI views optimized for dock panel layout
struct DockPanelOptimizedView<Content: View>: View {
    let content: Content
    
    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }
    
    var body: some View {
        content
            .frame(
                minWidth: PanelConstraints.minWidth,
                maxWidth: PanelConstraints.maxWidth,
                minHeight: PanelConstraints.minHeight,
                maxHeight: PanelConstraints.maxHeight
            )
            .background(Color.clear) // Transparent background for blur effect
    }
}