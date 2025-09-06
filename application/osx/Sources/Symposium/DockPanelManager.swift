import AppKit
import SwiftUI

/// Manages the lifecycle, positioning, and behavior of the dock-activated panel
class DockPanelManager: ObservableObject {
    
    /// Current panel instance (nil when hidden)
    private var currentPanel: DockPanel?
    
    /// Current hosting view for SwiftUI content
    private var currentHostingView: DockPanelHostingView<ProjectView>?
    
    /// Track panel visibility state
    @Published private(set) var isPanelVisible = false
    
    /// Click-outside monitor for dismissing panel
    private var clickOutsideMonitor: Any?
    
    init() {
        // Set up click-outside monitoring when needed
    }
    
    deinit {
        hidePanel()
    }
    
    // MARK: - Panel Display
    
    /// Show panel with project content at the specified location
    func showPanel(with projectManager: ProjectManager, near dockClickPoint: NSPoint, onCloseProject: (() -> Void)? = nil) {
        Logger.shared.log("DockPanelManager: showPanel called")
        Logger.shared.log("DockPanelManager: Dock click point: \(dockClickPoint)")
        Logger.shared.log("DockPanelManager: Project: \(projectManager.currentProject?.name ?? "nil")")
        Logger.shared.log("DockPanelManager: Current panel visible: \(isPanelVisible)")
        
        // Hide any existing panel first
        if isPanelVisible {
            Logger.shared.log("DockPanelManager: Hiding existing panel first")
            hidePanel()
        }
        
        // Create new panel and hosting view
        Logger.shared.log("DockPanelManager: Calculating ideal panel size")
        let idealSize = calculateIdealPanelSize()
        Logger.shared.log("DockPanelManager: Ideal panel size: \(idealSize)")
        
        let panelRect = NSRect(origin: .zero, size: idealSize)
        Logger.shared.log("DockPanelManager: Creating DockPanel with rect: \(panelRect)")
        
        let panel = DockPanel(
            contentRect: panelRect,
            styleMask: [.nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        
        // Create SwiftUI hosting view
        Logger.shared.log("DockPanelManager: Creating DockPanelHostingView")
        Logger.shared.log("DockPanelManager: Close callback provided: \(onCloseProject != nil)")
        let hostingView = DockPanelHostingView(projectManager: projectManager, onCloseProject: onCloseProject)
        
        // Set up the panel layout
        Logger.shared.log("DockPanelManager: Setting up panel layout")
        setupPanelLayout(panel: panel, hostingView: hostingView)
        
        // Calculate optimal position and arrow direction
        Logger.shared.log("DockPanelManager: Calculating panel position")
        let (panelPosition, arrowDirection, arrowPosition) = calculatePanelPosition(
            for: idealSize,
            near: dockClickPoint
        )
        Logger.shared.log("DockPanelManager: Panel position: \(panelPosition)")
        Logger.shared.log("DockPanelManager: Arrow direction: \(arrowDirection), position: \(arrowPosition)")
        
        // Configure arrow
        Logger.shared.log("DockPanelManager: Configuring panel arrow")
        panel.setArrowDirection(arrowDirection, position: arrowPosition)
        
        // Store references
        self.currentPanel = panel
        self.currentHostingView = hostingView
        Logger.shared.log("DockPanelManager: Stored panel and hosting view references")
        
        // Show panel with animation
        Logger.shared.log("DockPanelManager: Showing panel with animation")
        panel.showPanel(at: panelPosition)
        
        // Set up click-outside monitoring
        Logger.shared.log("DockPanelManager: Setting up click-outside monitoring")
        setupClickOutsideMonitoring()
        
        // Update state
        DispatchQueue.main.async {
            self.isPanelVisible = true
            Logger.shared.log("DockPanelManager: Panel visibility state updated to true")
        }
    }
    
    /// Hide the current panel
    func hidePanel() {
        Logger.shared.log("DockPanelManager: hidePanel called")
        
        guard let panel = currentPanel else { 
            Logger.shared.log("DockPanelManager: No current panel to hide")
            return 
        }
        
        Logger.shared.log("DockPanelManager: Hiding panel")
        
        // Remove click-outside monitoring
        if let monitor = clickOutsideMonitor {
            Logger.shared.log("DockPanelManager: Removing click-outside monitor")
            NSEvent.removeMonitor(monitor)
            clickOutsideMonitor = nil
        }
        
        // Hide panel with animation
        Logger.shared.log("DockPanelManager: Starting panel hide animation")
        panel.hidePanel { [weak self] in
            Logger.shared.log("DockPanelManager: Panel hide animation completed")
            self?.currentPanel = nil
            self?.currentHostingView = nil
        }
        
        // Update state immediately
        DispatchQueue.main.async {
            self.isPanelVisible = false
            Logger.shared.log("DockPanelManager: Panel visibility state updated to false")
        }
    }
    
    /// Toggle panel visibility
    func togglePanel(with projectManager: ProjectManager, near dockClickPoint: NSPoint, onCloseProject: (() -> Void)? = nil) {
        Logger.shared.log("DockPanelManager: togglePanel called")
        Logger.shared.log("DockPanelManager: Current panel visible: \(isPanelVisible)")
        
        if isPanelVisible {
            Logger.shared.log("DockPanelManager: Panel is visible, hiding it")
            hidePanel()
        } else {
            Logger.shared.log("DockPanelManager: Panel is hidden, showing it")
            showPanel(with: projectManager, near: dockClickPoint, onCloseProject: onCloseProject)
        }
    }
    
    // MARK: - Panel Layout and Positioning
    
    private func setupPanelLayout(panel: DockPanel, hostingView: DockPanelHostingView<ProjectView>) {
        guard let containerView = panel.contentView else { return }
        
        containerView.addSubview(hostingView)
        hostingView.translatesAutoresizingMaskIntoConstraints = false
        
        // Add padding to account for arrow space and margins
        let padding: CGFloat = 16
        let arrowSpace: CGFloat = 12
        
        NSLayoutConstraint.activate([
            hostingView.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: padding),
            hostingView.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -padding),
            hostingView.topAnchor.constraint(equalTo: containerView.topAnchor, constant: padding + arrowSpace),
            hostingView.bottomAnchor.constraint(equalTo: containerView.bottomAnchor, constant: -padding)
        ])
    }
    
    private func calculateTaskspaceWidth() -> CGFloat {
        let screenshotWidth: CGFloat = 120
        
        // Measure sample Star Trek log message
        let sampleText = "Captain, we're getting mysterious sensor readings. It seems like there's a wormhole appearing!"
        let textAttributes = [NSAttributedString.Key.font: NSFont.systemFont(ofSize: 13)]
        let sampleTextWidth = sampleText.size(withAttributes: textAttributes).width
        
        let padding: CGFloat = 40 // Internal card padding
        return screenshotWidth + sampleTextWidth + padding
    }
    
    private func calculateIdealPanelSize() -> NSSize {
        guard let screen = NSScreen.main else { return NSSize(width: 550, height: 800) }
        
        let taskspaceWidth = calculateTaskspaceWidth()
        let screenFrame = screen.visibleFrame
        
        // Panel Width Constraint Chain
        let idealWidth = 4 * taskspaceWidth              // Target 4 taskspaces per row
        let screenConstraint = 0.75 * screenFrame.width  // Max 3/4 screen width  
        let constrainedWidth = min(idealWidth, screenConstraint)
        let finalWidth = max(constrainedWidth, taskspaceWidth) // Min 1 taskspace width
        let panelWidth = min(finalWidth, screenFrame.width)    // Hard screen limit
        
        // Calculate height (placeholder logic)
        let panelHeight = min(800, 0.8 * screenFrame.height)
        
        return NSSize(width: panelWidth, height: panelHeight)
    }
    
    private func calculatePanelPosition(for panelSize: NSSize, near dockClickPoint: NSPoint) -> (position: NSPoint, arrowDirection: DockPanel.ArrowDirection, arrowPosition: CGFloat) {
        guard let screen = NSScreen.main else { return (NSPoint.zero, .down, 0.5) }
        let screenFrame = screen.visibleFrame
        
        // Center the panel on screen
        let centeredX = screenFrame.midX - (panelSize.width / 2)
        let centeredY = screenFrame.midY - (panelSize.height / 2)
        
        return (NSPoint(x: centeredX, y: centeredY), .down, 0.5)
    }
    
    // MARK: - Dock Location Detection
    // (Removed - no longer needed for centered positioning)
    
    // MARK: - Click Outside Monitoring
    
    private func setupClickOutsideMonitoring() {
        clickOutsideMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] event in
            guard let self = self, let panel = self.currentPanel else { return }
            
            let clickLocation = event.locationInWindow
            let panelFrame = panel.frame
            
            // Check if click is outside panel bounds
            if !panelFrame.contains(clickLocation) {
                DispatchQueue.main.async {
                    self.hidePanel()
                }
            }
        }
    }
    
    // MARK: - Panel Updates
    
    /// Update panel content with new project manager (for project switching)
    func updatePanelContent(with projectManager: ProjectManager) {
        guard let hostingView = currentHostingView else { return }
        hostingView.updateProjectManager(projectManager)
    }
}