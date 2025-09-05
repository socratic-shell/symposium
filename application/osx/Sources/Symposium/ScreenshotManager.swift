import AppKit
import CoreGraphics
import Foundation
import ScreenCaptureKit

/// Utility for capturing window screenshots using ScreenCaptureKit
///
/// Simplified stateless service - all caching handled by ProjectManager
class ScreenshotManager {

    private let permissionManager: PermissionManager

    init(permissionManager: PermissionManager) {
        self.permissionManager = permissionManager
    }

    /// Check if we can capture screenshots (requires Screen Recording permission)
    var canCaptureScreenshots: Bool {
        return permissionManager.hasScreenRecordingPermission
    }

    /// Capture screenshot of a window by CGWindowID and return it directly
    func captureWindowScreenshot(windowId: CGWindowID) async -> NSImage? {
        let startTime = Date()
        Logger.shared.log("ScreenshotManager: Attempting to capture window \(windowId)")
        Logger.shared.log("ScreenshotManager: Can capture screenshots: \(canCaptureScreenshots)")

        guard canCaptureScreenshots else {
            Logger.shared.log(
                "ScreenshotManager: Screenshot capture failed: Missing Screen Recording permission")
            return nil
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
                return nil
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
            let captureStartTime = Date()
            let cgImage = try await SCScreenshotManager.captureImage(
                contentFilter: filter, configuration: configuration)

            // Convert to NSImage and return
            let conversionStartTime = Date()
            Logger.shared.log("ScreenshotManager: Convert to NSImage and return")
            let screenshot = NSImage(
                cgImage: cgImage, size: NSSize(width: cgImage.width, height: cgImage.height))

            let endTime = Date()
            let totalTime = endTime.timeIntervalSince(startTime)
            let conversionTime = endTime.timeIntervalSince(conversionStartTime)
            let captureTime = conversionStartTime.timeIntervalSince(captureStartTime)
            Logger.shared.log(
                "ScreenshotManager: Screenshot captured successfully (total: \(String(format: "%.3f", totalTime))s, capture: \(String(format: "%.3f", captureTime)), conversion: \(String(format: "%.3f", conversionTime))s)"
            )
            return screenshot

        } catch {
            let errorTime = Date().timeIntervalSince(startTime)
            Logger.shared.log(
                "ScreenshotManager: Failed to capture screenshot after \(String(format: "%.3f", errorTime))s: \(error)"
            )
            return nil
        }
    }
}
