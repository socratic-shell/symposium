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
        Logger.shared.log("DockPanelHostingView: Panel constraints - width: \(PanelConstraints.minWidth)-\(PanelConstraints.maxWidth), height: \(PanelConstraints.minHeight)-\(PanelConstraints.maxHeight)")
        
        // Configure hosting view for panel layout
        self.translatesAutoresizingMaskIntoConstraints = false
        
        // Set initial size constraints
        self.widthAnchor.constraint(greaterThanOrEqualToConstant: PanelConstraints.minWidth).isActive = true
        self.widthAnchor.constraint(lessThanOrEqualToConstant: PanelConstraints.maxWidth).isActive = true
        self.heightAnchor.constraint(greaterThanOrEqualToConstant: PanelConstraints.minHeight).isActive = true
        self.heightAnchor.constraint(lessThanOrEqualToConstant: PanelConstraints.maxHeight).isActive = true
        
        // Prefer the default width
        let widthConstraint = self.widthAnchor.constraint(equalToConstant: PanelConstraints.defaultWidth)
        widthConstraint.priority = NSLayoutConstraint.Priority(999)
        widthConstraint.isActive = true
        
        Logger.shared.log("DockPanelHostingView: Constraints configured successfully")
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
    convenience init(projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil) {
        let projectView = ProjectView(projectManager: projectManager, onCloseProject: onCloseProject)
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