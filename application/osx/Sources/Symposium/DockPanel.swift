import AppKit
import SwiftUI

/// Custom NSPanel that displays taskspace information in a dock-activated floating panel
class DockPanel: NSPanel {
    
    /// Direction the arrow should point (toward the dock)
    enum ArrowDirection {
        case up, down, left, right
    }
    
    private var arrowDirection: ArrowDirection = .down
    private var arrowPosition: CGFloat = 0.5 // 0.0 to 1.0 along the edge
    
    override init(contentRect: NSRect, styleMask style: NSWindow.StyleMask, backing backingStoreType: NSWindow.BackingStoreType, defer flag: Bool) {
        Logger.shared.log("DockPanel: Initializing with contentRect: \(contentRect)")
        super.init(contentRect: contentRect, styleMask: style, backing: backingStoreType, defer: flag)
        
        Logger.shared.log("DockPanel: Setting up panel configuration")
        setupPanel()
    }
    
    private func setupPanel() {
        Logger.shared.log("DockPanel: Configuring panel behavior")
        // Configure panel behavior
        self.styleMask = [.nonactivatingPanel, .resizable]
        self.level = .floating
        self.hidesOnDeactivate = false
        self.hasShadow = true
        self.isOpaque = false
        self.backgroundColor = NSColor.clear
        
        Logger.shared.log("DockPanel: Configuring panel appearance")
        // Configure panel appearance
        self.titlebarAppearsTransparent = true
        self.titleVisibility = .hidden
        self.isMovableByWindowBackground = true
        
        Logger.shared.log("DockPanel: Setting up visual effect view")
        // Set up visual effect view for blur background
        setupVisualEffectView()
    }
    
    private func setupVisualEffectView() {
        let visualEffectView = NSVisualEffectView()
        visualEffectView.material = .menu // System menu material
        visualEffectView.blendingMode = .behindWindow
        visualEffectView.state = .active
        
        // Create a custom view that handles the rounded corners and arrow
        let containerView = DockPanelContainerView(arrowDirection: arrowDirection, arrowPosition: arrowPosition)
        containerView.translatesAutoresizingMaskIntoConstraints = false
        
        // Add visual effect as background
        containerView.addSubview(visualEffectView, positioned: .below, relativeTo: nil)
        visualEffectView.translatesAutoresizingMaskIntoConstraints = false
        
        NSLayoutConstraint.activate([
            visualEffectView.leadingAnchor.constraint(equalTo: containerView.leadingAnchor),
            visualEffectView.trailingAnchor.constraint(equalTo: containerView.trailingAnchor),
            visualEffectView.topAnchor.constraint(equalTo: containerView.topAnchor),
            visualEffectView.bottomAnchor.constraint(equalTo: containerView.bottomAnchor)
        ])
        
        // Set the container as the content view
        self.contentView = containerView
    }
    
    /// Update arrow direction and position based on dock location
    func setArrowDirection(_ direction: ArrowDirection, position: CGFloat = 0.5) {
        Logger.shared.log("DockPanel: Setting arrow direction: \(direction), position: \(position)")
        self.arrowDirection = direction
        self.arrowPosition = max(0.0, min(1.0, position))
        
        // Update the container view
        if let containerView = contentView as? DockPanelContainerView {
            Logger.shared.log("DockPanel: Updating container view arrow")
            containerView.updateArrow(direction: direction, position: position)
        } else {
            Logger.shared.log("DockPanel: WARNING - Content view is not DockPanelContainerView")
        }
    }
    
    /// Show panel with animation
    func showPanel(at point: NSPoint) {
        Logger.shared.log("DockPanel: showPanel at point: \(point)")
        Logger.shared.log("DockPanel: Panel frame before: \(self.frame)")
        
        self.setFrameOrigin(point)
        self.alphaValue = 0.0
        
        Logger.shared.log("DockPanel: Making panel key and ordering front")
        self.makeKeyAndOrderFront(nil)
        
        Logger.shared.log("DockPanel: Starting fade-in animation")
        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.25
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            self.animator().alphaValue = 1.0
        }
        Logger.shared.log("DockPanel: Panel frame after: \(self.frame)")
    }
    
    /// Hide panel with animation
    func hidePanel(completion: (() -> Void)? = nil) {
        Logger.shared.log("DockPanel: hidePanel called")
        Logger.shared.log("DockPanel: Starting fade-out animation")
        
        NSAnimationContext.runAnimationGroup({ context in
            context.duration = 0.2
            context.timingFunction = CAMediaTimingFunction(name: .easeIn)
            self.animator().alphaValue = 0.0
        }, completionHandler: {
            Logger.shared.log("DockPanel: Fade-out animation completed, ordering out")
            self.orderOut(nil)
            completion?()
        })
    }
}

/// Custom container view that draws the rounded rectangle with arrow/tail
class DockPanelContainerView: NSView {
    
    private var arrowDirection: DockPanel.ArrowDirection
    private var arrowPosition: CGFloat
    private let arrowSize: CGFloat = 12.0
    private let cornerRadius: CGFloat = 8.0
    
    init(arrowDirection: DockPanel.ArrowDirection, arrowPosition: CGFloat) {
        self.arrowDirection = arrowDirection
        self.arrowPosition = arrowPosition
        super.init(frame: .zero)
        
        self.wantsLayer = true
        self.layer?.masksToBounds = false
    }
    
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
    
    func updateArrow(direction: DockPanel.ArrowDirection, position: CGFloat) {
        self.arrowDirection = direction
        self.arrowPosition = position
        self.needsDisplay = true
    }
    
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        
        guard let context = NSGraphicsContext.current?.cgContext else { return }
        
        // Create path for panel with arrow
        let path = createPanelPath()
        
        // Set up clipping path for the visual effect view
        context.addPath(path)
        context.clip()
    }
    
    override func updateLayer() {
        super.updateLayer()
        
        // Create the mask layer for rounded corners and arrow
        let maskLayer = CAShapeLayer()
        maskLayer.path = createPanelPath()
        self.layer?.mask = maskLayer
    }
    
    private func createPanelPath() -> CGPath {
        let bounds = self.bounds
        let path = CGMutablePath()
        
        // Calculate panel rect (excluding arrow space)
        var panelRect = bounds
        switch arrowDirection {
        case .up:
            panelRect.origin.y += arrowSize
            panelRect.size.height -= arrowSize
        case .down:
            panelRect.size.height -= arrowSize
        case .left:
            panelRect.origin.x += arrowSize
            panelRect.size.width -= arrowSize
        case .right:
            panelRect.size.width -= arrowSize
        }
        
        // Create rounded rectangle for main panel
        path.addRoundedRect(in: panelRect, cornerWidth: cornerRadius, cornerHeight: cornerRadius)
        
        // Add arrow triangle
        addArrowToPath(path, panelRect: panelRect)
        
        return path
    }
    
    private func addArrowToPath(_ path: CGMutablePath, panelRect: CGRect) {
        let arrowHalfWidth = arrowSize * 0.6
        
        switch arrowDirection {
        case .down:
            // Arrow pointing down from bottom edge
            let arrowCenterX = panelRect.minX + (panelRect.width * arrowPosition)
            let arrowTop = panelRect.minY
            let arrowBottom = bounds.minY
            
            path.move(to: CGPoint(x: arrowCenterX - arrowHalfWidth, y: arrowTop))
            path.addLine(to: CGPoint(x: arrowCenterX, y: arrowBottom))
            path.addLine(to: CGPoint(x: arrowCenterX + arrowHalfWidth, y: arrowTop))
            
        case .up:
            // Arrow pointing up from top edge
            let arrowCenterX = panelRect.minX + (panelRect.width * arrowPosition)
            let arrowBottom = panelRect.maxY
            let arrowTop = bounds.maxY
            
            path.move(to: CGPoint(x: arrowCenterX - arrowHalfWidth, y: arrowBottom))
            path.addLine(to: CGPoint(x: arrowCenterX, y: arrowTop))
            path.addLine(to: CGPoint(x: arrowCenterX + arrowHalfWidth, y: arrowBottom))
            
        case .left:
            // Arrow pointing left from left edge
            let arrowCenterY = panelRect.minY + (panelRect.height * arrowPosition)
            let arrowRight = panelRect.minX
            let arrowLeft = bounds.minX
            
            path.move(to: CGPoint(x: arrowRight, y: arrowCenterY - arrowHalfWidth))
            path.addLine(to: CGPoint(x: arrowLeft, y: arrowCenterY))
            path.addLine(to: CGPoint(x: arrowRight, y: arrowCenterY + arrowHalfWidth))
            
        case .right:
            // Arrow pointing right from right edge
            let arrowCenterY = panelRect.minY + (panelRect.height * arrowPosition)
            let arrowLeft = panelRect.maxX
            let arrowRight = bounds.maxX
            
            path.move(to: CGPoint(x: arrowLeft, y: arrowCenterY - arrowHalfWidth))
            path.addLine(to: CGPoint(x: arrowRight, y: arrowCenterY))
            path.addLine(to: CGPoint(x: arrowLeft, y: arrowCenterY + arrowHalfWidth))
        }
    }
}