import AppKit
import CoreGraphics
import Foundation
import ScreenCaptureKit

/// Manages window screenshots for taskspaces using ScreenCaptureKit
/// 
/// CURRENT STATUS (Phase 2.10): Core implementation complete but troubleshooting UI refresh issues
/// 
/// ISSUE: Screenshots showing as "Disconnected" in UI despite successful window registration
/// - Window associations are working (logs show successful CGWindowID associations)  
/// - Debug logging added throughout capture flow to identify bottleneck
/// - Made taskspaceWindows @Published to trigger UI updates
/// 
/// NEXT STEPS FOR TROUBLESHOOTING:
/// 1. Check debug logs for screenshot capture attempts:
///    - Look for "ProjectManager: Attempting screenshot capture" messages
///    - Look for "ScreenshotManager: Attempting to capture" messages  
///    - Look for "Screenshot captured for taskspace" success messages
/// 2. If no capture logs appear, issue is screenshot trigger not firing
/// 3. If capture logs appear but fail, issue is ScreenCaptureKit API calls
/// 4. If capture succeeds but UI doesn't update, issue is @Published propagation
/// 
/// IMPLEMENTATION DETAILS:
/// - Uses ScreenCaptureKit (macOS 14.0+ only) with proper availability checks
/// - Screenshots cached in @Published dictionary for automatic UI updates  
/// - Captures triggered on window registration AND log updates for live feedback
/// - Integrates with PermissionManager for Screen Recording permission
@available(macOS 14.0, *)
class ScreenshotManager: ObservableObject {

    /// Cache of screenshots by taskspace UUID (main actor only)
    @MainActor @Published private var screenshots: [UUID: NSImage] = [:]

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
        Logger.shared.log(
            "ScreenshotManager: Attempting to capture window \(windowId) for taskspace \(taskspaceId)"
        )
        Logger.shared.log("ScreenshotManager: Can capture screenshots: \(canCaptureScreenshots)")

        guard canCaptureScreenshots else {
            Logger.shared.log(
                "ScreenshotManager: Screenshot capture failed: Missing Screen Recording permission")
            return
        }

        do {
            // Get available content for screen capture
            Logger.shared.log("ScreenshotManager: Get available content for screen capture")
            let availableContent = try await SCShareableContent.excludingDesktopWindows(
                false, onScreenWindowsOnly: true)

            // Find the specific window we want to capture
            Logger.shared.log("ScreenshotManager: Find the specific window we want to capture")
            guard
                let targetWindow = availableContent.windows.first(where: { $0.windowID == windowId }
                )
            else {
                Logger.shared.log("ScreenshotManager: Window not found for screenshot: \(windowId)")
                return
            }

            // Create filter with just this window
            Logger.shared.log("ScreenshotManager: Create filter with just this window")
            let filter = SCContentFilter(desktopIndependentWindow: targetWindow)

            // Configure screenshot capture
            Logger.shared.log("ScreenshotManager: Configure screenshot capture")
            let configuration = SCStreamConfiguration()
            configuration.width = Int(targetWindow.frame.width)
            configuration.height = Int(targetWindow.frame.height)
            configuration.scalesToFit = true
            configuration.captureResolution = .automatic

            // Capture the screenshot
            Logger.shared.log("ScreenshotManager: Capture the screenshot")
            let cgImage = try await SCScreenshotManager.captureImage(
                contentFilter: filter, configuration: configuration)

            // Convert to NSImage
            Logger.shared.log("ScreenshotManager: Convert to NSImage")
            let screenshot = NSImage(
                cgImage: cgImage, size: NSSize(width: cgImage.width, height: cgImage.height))

            // Cache the screenshot on main actor
            Logger.shared.log("ScreenshotManager: Cache the screenshot on main actor")
            await MainActor.run {
                screenshots[taskspaceId] = screenshot
                screenshotTimestamps[taskspaceId] = Date()
            }

            Logger.shared.log(
                "ScreenshotManager: Screenshot captured for taskspace: \(taskspaceId)")

        } catch {
            Logger.shared.log("ScreenshotManager: Failed to capture screenshot: \(error)")
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
    func cleanupOldScreenshots(olderThan timeInterval: TimeInterval = 300) {  // 5 minutes
        let cutoffDate = Date().addingTimeInterval(-timeInterval)

        for (taskspaceId, timestamp) in screenshotTimestamps {
            if timestamp < cutoffDate {
                screenshots.removeValue(forKey: taskspaceId)
                screenshotTimestamps.removeValue(forKey: taskspaceId)
            }
        }
    }
}
