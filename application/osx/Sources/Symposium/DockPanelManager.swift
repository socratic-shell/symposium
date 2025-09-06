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
    
    /// Show panel with project content
    func showPanel(with projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil, onDismiss: (() -> Void)? = nil) {
        Logger.shared.log("DockPanelManager: showPanel called")
        Logger.shared.log("DockPanelManager: Project: \(projectManager.currentProject?.name ?? "nil")")
        Logger.shared.log("DockPanelManager: Current panel visible: \(isPanelVisible)")
        
        // Hide any existing panel first
        if isPanelVisible {
            Logger.shared.log("DockPanelManager: Hiding existing panel first")
            hidePanel()
        }
        
        // Create new panel and hosting view
        Logger.shared.log("DockPanelManager: Calculating ideal panel size")
        let idealSize = calculateIdealPanelSize(for: projectManager)
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
        Logger.shared.log("DockPanelManager: Dismiss callback provided: \(onDismiss != nil)")
        let hostingView = DockPanelHostingView(projectManager: projectManager, onCloseProject: onCloseProject, onDismiss: onDismiss)
        
        // Set up the panel layout
        Logger.shared.log("DockPanelManager: Setting up panel layout")
        setupPanelLayout(panel: panel, hostingView: hostingView)
        
        // Calculate panel position
        Logger.shared.log("DockPanelManager: Calculating panel position")
        let panelPosition = calculatePanelPosition(for: idealSize)
        Logger.shared.log("DockPanelManager: Panel position: \(panelPosition)")
        
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
    func togglePanel(with projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil, onDismiss: (() -> Void)? = nil) {
        Logger.shared.log("DockPanelManager: togglePanel called")
        Logger.shared.log("DockPanelManager: Current panel visible: \(isPanelVisible)")
        
        if isPanelVisible {
            Logger.shared.log("DockPanelManager: Panel is visible, hiding it")
            hidePanel()
        } else {
            Logger.shared.log("DockPanelManager: Panel is hidden, showing it")
            showPanel(with: projectManager, onCloseProject: onCloseProject, onDismiss: onDismiss)
        }
    }
    
    // MARK: - Panel Layout and Positioning
    
    private func setupPanelLayout(panel: DockPanel, hostingView: DockPanelHostingView<ProjectView>) {
        guard let containerView = panel.contentView else { return }
        
        containerView.addSubview(hostingView)
        hostingView.translatesAutoresizingMaskIntoConstraints = false
        
        // Add padding for clean margins (no arrow space needed)
        let padding: CGFloat = 16
        
        NSLayoutConstraint.activate([
            hostingView.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: padding),
            hostingView.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -padding),
            hostingView.topAnchor.constraint(equalTo: containerView.topAnchor, constant: padding),
            hostingView.bottomAnchor.constraint(equalTo: containerView.bottomAnchor, constant: -padding)
        ])
    }
    
    private func calculateTaskspaceWidth() -> CGFloat {
        let screenshotWidth: CGFloat = 120
        
        // Measure sample Star Trek log message
        let sampleText = "Captain, we're getting mysterious sensor readings"
        let textAttributes = [NSAttributedString.Key.font: NSFont.systemFont(ofSize: 13)]
        let sampleTextWidth = sampleText.size(withAttributes: textAttributes).width
        
        let padding: CGFloat = 40 // Internal card padding
        return screenshotWidth + sampleTextWidth + padding
    }
    
    private func calculateIdealPanelSize(for projectManager: ProjectManager) -> NSSize {
        guard let screen = NSScreen.main else { return NSSize(width: 550, height: 800) }
        
        let taskspaceWidth = calculateTaskspaceWidth()
        let screenFrame = screen.visibleFrame
        
        // Panel Width Constraint Chain
        let idealWidth = 4 * taskspaceWidth              // Target 4 taskspaces per row
        let screenConstraint = 0.75 * screenFrame.width  // Max 3/4 screen width  
        let constrainedWidth = min(idealWidth, screenConstraint)
        let finalWidth = max(constrainedWidth, taskspaceWidth) // Min 1 taskspace width
        let panelWidth = min(finalWidth, screenFrame.width)    // Hard screen limit
        
        // Calculate responsive height with partial visibility
        let panelHeight = calculatePanelHeight(
            panelWidth: panelWidth,
            taskspaceWidth: taskspaceWidth, 
            taskspaceCount: projectManager.currentProject?.taskspaces.count ?? 0,
            screenHeight: screenFrame.height
        )
        
        return NSSize(width: panelWidth, height: panelHeight)
    }
    
    private func calculatePanelHeight(panelWidth: CGFloat, taskspaceWidth: CGFloat, taskspaceCount: Int, screenHeight: CGFloat) -> CGFloat {
        // Calculate grid dimensions
        let taskspacesPerRow = max(1, Int(floor(panelWidth / taskspaceWidth)))
        let totalRows = taskspaceCount == 0 ? 1 : Int(ceil(Double(taskspaceCount) / Double(taskspacesPerRow)))
        
        // Height components
        let taskspaceHeight: CGFloat = 160  // Approximate height of TaskspaceCard
        let headerHeight: CGFloat = 80      // Project header
        let footerHeight: CGFloat = 60      // "New Taskspace" footer  
        let spacing: CGFloat = 16           // Grid spacing
        let padding: CGFloat = 32           // Top/bottom padding
        
        // Calculate content height for all rows
        let contentHeight = CGFloat(totalRows) * taskspaceHeight + CGFloat(max(0, totalRows - 1)) * spacing
        let totalIdealHeight = headerHeight + contentHeight + footerHeight + padding
        
        // Screen constraints
        let maxHeight = 0.8 * screenHeight
        let constrainedHeight = min(totalIdealHeight, maxHeight)
        
        // If we're constrained and have more than 2 rows, show partial visibility
        if constrainedHeight < totalIdealHeight && totalRows > 2 {
            // Show complete rows minus ~30px to reveal part of the next row
            let partialVisibilityOffset: CGFloat = 30
            return constrainedHeight - partialVisibilityOffset
        }
        
        return constrainedHeight
    }
    
    private func calculatePanelPosition(for panelSize: NSSize) -> NSPoint {
        guard let screen = NSScreen.main else { return NSPoint.zero }
        let screenFrame = screen.visibleFrame
        
        // Center the panel on screen
        let centeredX = screenFrame.midX - (panelSize.width / 2)
        let centeredY = screenFrame.midY - (panelSize.height / 2)
        
        return NSPoint(x: centeredX, y: centeredY)
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