import AppKit
import SwiftUI

/// Custom NSPanel that displays taskspace information in a centered floating panel
class DockPanel: NSPanel {
    
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
        visualEffectView.wantsLayer = true
        visualEffectView.layer?.cornerRadius = 8.0
        visualEffectView.layer?.masksToBounds = true
        
        // Set the visual effect view as the content view
        self.contentView = visualEffectView
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

