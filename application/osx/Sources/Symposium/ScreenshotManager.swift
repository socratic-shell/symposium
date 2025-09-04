import Foundation
import AppKit
import ScreenCaptureKit
import CoreGraphics

/// Manages window screenshots for taskspaces using ScreenCaptureKit
@available(macOS 14.0, *)
class ScreenshotManager: ObservableObject {
    
    /// Cache of screenshots by taskspace UUID (main actor only)
    @MainActor private var screenshots: [UUID: NSImage] = [:]
    
    /// Track screenshot update times to manage cache
    private var screenshotTimestamps: [UUID: Date] = [:]
    
    private let permissionManager: PermissionManager
    
    init(permissionManager: PermissionManager) {
        self.permissionManager = permissionManager
    }
    
    /// Check if we can capture screenshots (requires Screen Recording permission)
    var canCaptureScreenshots: Bool {
        return permissionManager.hasScreenRecordingPermission
    }
    
    /// Get cached screenshot for a taskspace
    @MainActor
    func getScreenshot(for taskspaceId: UUID) -> NSImage? {
        return screenshots[taskspaceId]
    }
    
    /// Capture screenshot of a window by CGWindowID
    func captureWindowScreenshot(windowId: CGWindowID, for taskspaceId: UUID) async {
        guard canCaptureScreenshots else {
            Logger.shared.log("Screenshot capture failed: Missing Screen Recording permission")
            return
        }
        
        do {
            // Get available content for screen capture
            let availableContent = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            // Find the specific window we want to capture
            guard let targetWindow = availableContent.windows.first(where: { $0.windowID == windowId }) else {
                Logger.shared.log("Window not found for screenshot: \(windowId)")
                return
            }
            
            // Create filter with just this window
            let filter = SCContentFilter(desktopIndependentWindow: targetWindow)
            
            // Configure screenshot capture
            let configuration = SCStreamConfiguration()
            configuration.width = Int(targetWindow.frame.width)
            configuration.height = Int(targetWindow.frame.height)
            configuration.scalesToFit = true
            configuration.captureResolution = .automatic
            
            // Capture the screenshot
            let cgImage = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
            
            // Convert to NSImage
            let screenshot = NSImage(cgImage: cgImage, size: NSSize(width: cgImage.width, height: cgImage.height))
            
            // Cache the screenshot on main actor
            await MainActor.run {
                screenshots[taskspaceId] = screenshot
                screenshotTimestamps[taskspaceId] = Date()
            }
            
            Logger.shared.log("Screenshot captured for taskspace: \(taskspaceId)")
            
        } catch {
            Logger.shared.log("Failed to capture screenshot: \(error)")
        }
    }
    
    /// Remove screenshot from cache when taskspace is disconnected
    @MainActor
    func removeScreenshot(for taskspaceId: UUID) {
        screenshots.removeValue(forKey: taskspaceId)
        screenshotTimestamps.removeValue(forKey: taskspaceId)
    }
    
    /// Clear old screenshots to manage memory usage
    @MainActor
    func cleanupOldScreenshots(olderThan timeInterval: TimeInterval = 300) { // 5 minutes
        let cutoffDate = Date().addingTimeInterval(-timeInterval)
        
        for (taskspaceId, timestamp) in screenshotTimestamps {
            if timestamp < cutoffDate {
                screenshots.removeValue(forKey: taskspaceId)
                screenshotTimestamps.removeValue(forKey: taskspaceId)
            }
        }
    }
}