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
    
    private func calculateIdealPanelSize() -> NSSize {
        // Use a larger, more spacious panel size for better information density
        // Width increased for better horizontal layout, height maintained for screen compatibility
        return NSSize(width: 550, height: 800)
    }
    
    private func calculatePanelPosition(for panelSize: NSSize, near dockClickPoint: NSPoint) -> (position: NSPoint, arrowDirection: DockPanel.ArrowDirection, arrowPosition: CGFloat) {
        
        guard let screen = NSScreen.main else {
            return (dockClickPoint, .down, 0.5)
        }
        
        let screenFrame = screen.visibleFrame
        let dockLocation = determineDockLocation(screenFrame: screenFrame)
        
        let margin: CGFloat = 20
        var panelPosition: NSPoint
        var arrowDirection: DockPanel.ArrowDirection
        var arrowPosition: CGFloat = 0.5
        
        switch dockLocation {
        case .bottom:
            // Panel appears above dock
            panelPosition = NSPoint(
                x: dockClickPoint.x - panelSize.width / 2,
                y: screenFrame.minY + margin
            )
            arrowDirection = .down
            
            // Ensure panel stays on screen
            panelPosition.x = max(screenFrame.minX + margin, 
                                min(panelPosition.x, screenFrame.maxX - panelSize.width - margin))
            
            // Calculate arrow position relative to panel
            arrowPosition = (dockClickPoint.x - panelPosition.x) / panelSize.width
            arrowPosition = max(0.1, min(0.9, arrowPosition))
            
        case .left:
            // Panel appears to right of dock
            panelPosition = NSPoint(
                x: screenFrame.minX + margin,
                y: dockClickPoint.y - panelSize.height / 2
            )
            arrowDirection = .left
            
            // Ensure panel stays on screen vertically
            panelPosition.y = max(screenFrame.minY + margin,
                                min(panelPosition.y, screenFrame.maxY - panelSize.height - margin))
            
            arrowPosition = (dockClickPoint.y - panelPosition.y) / panelSize.height
            arrowPosition = max(0.1, min(0.9, arrowPosition))
            
        case .right:
            // Panel appears to left of dock
            panelPosition = NSPoint(
                x: screenFrame.maxX - panelSize.width - margin,
                y: dockClickPoint.y - panelSize.height / 2
            )
            arrowDirection = .right
            
            panelPosition.y = max(screenFrame.minY + margin,
                                min(panelPosition.y, screenFrame.maxY - panelSize.height - margin))
            
            arrowPosition = (dockClickPoint.y - panelPosition.y) / panelSize.height
            arrowPosition = max(0.1, min(0.9, arrowPosition))
        }
        
        return (panelPosition, arrowDirection, arrowPosition)
    }
    
    // MARK: - Dock Location Detection
    
    enum DockLocation {
        case bottom, left, right
    }
    
    private func determineDockLocation(screenFrame: NSRect) -> DockLocation {
        // Simple heuristic: check dock preferences or assume bottom
        // For MVP, we'll assume bottom dock (most common)
        // TODO: Implement actual dock position detection
        return .bottom
    }
    
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