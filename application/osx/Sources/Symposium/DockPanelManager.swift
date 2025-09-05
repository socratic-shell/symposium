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
    func showPanel(with projectManager: ProjectManager, near dockClickPoint: NSPoint) {
        // Hide any existing panel first
        if isPanelVisible {
            hidePanel()
        }
        
        // Create new panel and hosting view
        let idealSize = calculateIdealPanelSize()
        let panelRect = NSRect(origin: .zero, size: idealSize)
        
        let panel = DockPanel(
            contentRect: panelRect,
            styleMask: [.nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        
        // Create SwiftUI hosting view
        let hostingView = DockPanelHostingView(projectManager: projectManager)
        
        // Set up the panel layout
        setupPanelLayout(panel: panel, hostingView: hostingView)
        
        // Calculate optimal position and arrow direction
        let (panelPosition, arrowDirection, arrowPosition) = calculatePanelPosition(
            for: idealSize,
            near: dockClickPoint
        )
        
        // Configure arrow
        panel.setArrowDirection(arrowDirection, position: arrowPosition)
        
        // Store references
        self.currentPanel = panel
        self.currentHostingView = hostingView
        
        // Show panel with animation
        panel.showPanel(at: panelPosition)
        
        // Set up click-outside monitoring
        setupClickOutsideMonitoring()
        
        // Update state
        DispatchQueue.main.async {
            self.isPanelVisible = true
        }
    }
    
    /// Hide the current panel
    func hidePanel() {
        guard let panel = currentPanel else { return }
        
        // Remove click-outside monitoring
        if let monitor = clickOutsideMonitor {
            NSEvent.removeMonitor(monitor)
            clickOutsideMonitor = nil
        }
        
        // Hide panel with animation
        panel.hidePanel { [weak self] in
            self?.currentPanel = nil
            self?.currentHostingView = nil
        }
        
        // Update state immediately
        DispatchQueue.main.async {
            self.isPanelVisible = false
        }
    }
    
    /// Toggle panel visibility
    func togglePanel(with projectManager: ProjectManager, near dockClickPoint: NSPoint) {
        if isPanelVisible {
            hidePanel()
        } else {
            showPanel(with: projectManager, near: dockClickPoint)
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
        let hostingView = DockPanelHostingView<ProjectView>.init(rootView: ProjectView(projectManager: ProjectManager(agentManager: AgentManager(), settingsManager: SettingsManager(), selectedAgent: .claude, permissionManager: PermissionManager())))
        return hostingView.calculateIdealSize()
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