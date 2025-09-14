import AppKit
import SwiftUI

@main
struct SymposiumApp: App {
    @StateObject private var agentManager = AgentManager()
    @StateObject private var settingsManager = SettingsManager()
    @StateObject private var permissionManager = PermissionManager()

    // App delegate for dock click handling
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    // SwiftUI environment for window management
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismissWindow) private var dismissWindow

    var body: some Scene {
        // Splash window - shown first on app launch with cute loading messages
        WindowGroup(id: "splash") {
            SplashView()
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
                .environmentObject(appDelegate)
                .onAppear {
                    Logger.shared.log("Splash window opened - running startup logic")
                    appStart()
                }
        }
        .windowResizability(.contentSize)

        // Project selection window
        WindowGroup(id: "choose-project") {
            ProjectSelectionView(onProjectSelected: handleProjectSelected)
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
                .onAppear {
                    Logger.shared.log("Project selection window opened")
                }
                .onDisappear {
                    Logger.shared.log("Project selection window closed")
                    appStart()
                }
        }
        .windowResizability(.contentSize)

        // Settings window
        WindowGroup(id: "settings") {
            SettingsView()
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
                .onAppear {
                    Logger.shared.log("Settings window opened")
                }
                .onDisappear {
                    Logger.shared.log("Settings window closed")
                    appStart()
                }
        }
        .windowResizability(.contentSize)

        // Main project window
        WindowGroup(id: "open-project") {
            ProjectWindow()
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
                .onAppear {
                    Logger.shared.log("Project window opened")
                }
                .onReceive(
                    NotificationCenter.default.publisher(for: NSWindow.willCloseNotification)
                ) { notification in
                    Logger.shared.log("Received NSWindow.willCloseNotification")

                    if let window = notification.object as? NSWindow,
                        let identifier = window.identifier?.rawValue,
                        identifier.hasPrefix("open-project")
                    {
                        Logger.shared.log(
                            "Project window explicitly closed by user (identifier: \(identifier))")
                        appDelegate.currentProjectManager = nil
                        settingsManager.activeProjectPath = ""
                        appStart()
                    } else {
                        Logger.shared.log("Window close notification for different window")
                    }
                }
                .onDisappear {
                    // NOTE: We don't handle project cleanup here because onDisappear
                    // fires both when user closes window AND when app quits.
                    // We only want to clear the project on explicit user close,
                    // so we use NSWindow.willCloseNotification above instead.
                    Logger.shared.log("Project window disappeared")
                }
        }
        .windowResizability(.contentSize)
        .defaultAppStorage(.standard)

        .commands {
            // File menu items
            CommandGroup(replacing: .newItem) {
                Button("New Project...") {
                    showProjectSelectionWindow()
                }
                .keyboardShortcut("n", modifiers: .command)

                Button("Open Project...") {
                    showProjectSelectionWindow()
                }
                .keyboardShortcut("o", modifiers: .command)
            }

            CommandGroup(after: .help) {
                Button("Copy Debug Logs") {
                    copyLogsToClipboard()
                }
                .keyboardShortcut("d", modifiers: [.command, .shift])

                Button("List All Windows") {
                    listAllWindows()
                }
                .keyboardShortcut("w", modifiers: [.command, .shift])
            }
        }

        Settings {
            SettingsView()
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
        }
    }

    private func openProjectWindow(with projectManager: ProjectManager) {
        Logger.shared.log("App: Setting current project manager and opening project window")
        appDelegate.currentProjectManager = projectManager
        // Dismiss project selection window if it's open
        dismissWindow(id: "choose-project")
        openWindow(id: "open-project")
    }

    private func handleProjectSelected(_ path: String) {
        Logger.shared.log("App: Project selected at path: \(path)")
        settingsManager.activeProjectPath = path

        // Delay closing the window to let the sheet dismiss first
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            self.closeWindow(id: "choose-project")
        }
        // appStart() will be called by onDisappear
    }

    private func closeWindow(id: String) {
        Logger.shared.log("App: Closing window with id: \(id)")
        dismissWindow(id: id)
    }

    private func dismissSplash() {
        Logger.shared.log("App: Dismissing splash window")
        closeWindow(id: "splash")
    }

    private func reregisterWindows(for projectManager: ProjectManager) {
        guard let project = projectManager.currentProject else {
            Logger.shared.log("App: No current project for window re-registration")
            return
        }

        Logger.shared.log("App: Re-registering windows for \(project.taskspaces.count) taskspaces")

        for taskspace in project.taskspaces {
            // Send taskspace roll call message
            let payload = TaskspaceRollCallPayload(taskspaceUuid: taskspace.id.uuidString)
            projectManager.mcpStatus.sendBroadcastMessage(
                type: "taskspace_roll_call", payload: payload)
            Logger.shared.log("App: Sent roll call for taskspace: \(taskspace.name)")
        }
    }

    private func showProjectSelectionWindow() {
        Logger.shared.log("App: Opening project selection window")
        openWindow(id: "choose-project")
    }

    /// Startup state machine - determines which window to show based on app state
    private func appStart() {
        Logger.shared.log("App: Starting appStart() state machine")

        // Give splash window time to appear, then run logic
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            self.runStartupLogic()
        }
    }

    private func runStartupLogic() {
        Logger.shared.log("App: Running startup logic checks")

        // Check 1: Do we have required permissions?
        let hasPermissions =
            permissionManager.hasAccessibilityPermission
            && permissionManager.hasScreenRecordingPermission

        if !hasPermissions {
            Logger.shared.log("App: Missing permissions, showing settings window")
            dismissSplash()
            openWindow(id: "settings")
            return
        }

        // Check 2: Are agents ready? (needed for project restoration)
        if !agentManager.scanningCompleted {
            Logger.shared.log("App: Agent scan not complete, waiting...")
            // Wait a bit and try again
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                self.runStartupLogic()
            }
            return
        }

        // Check 3: Is there a previously opened project?
        if !settingsManager.activeProjectPath.isEmpty,
            isValidProject(at: settingsManager.activeProjectPath)
        {
            Logger.shared.log(
                "App: Found valid last project at \(settingsManager.activeProjectPath), attempting to restore"
            )
            dismissSplash()
            restoreLastProject(at: settingsManager.activeProjectPath)
            return
        }

        // Default: Show project selection
        Logger.shared.log("App: No previous project, showing project selection window")
        dismissSplash()
        openWindow(id: "choose-project")
    }

    private func restoreLastProject(at path: String) {
        do {
            let projectManager = ProjectManager(
                agentManager: agentManager,
                settingsManager: settingsManager,
                selectedAgent: settingsManager.selectedAgent,
                permissionManager: permissionManager
            )
            try projectManager.openProject(at: path)
            openProjectWindow(with: projectManager)

            // Automatically refresh window connections on startup
            Logger.shared.log("App: Auto-refreshing window connections after project restoration")
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
                self.reregisterWindows(for: projectManager)
            }

            Logger.shared.log("App: Successfully restored last project")
        } catch {
            Logger.shared.log(
                "App: Failed to restore last project: \(error), showing project selection")
            // Clear invalid project path
            settingsManager.activeProjectPath = ""
            dismissSplash()
            openWindow(id: "choose-project")
        }
    }

    private func isValidProject(at path: String) -> Bool {
        // Use the same validation logic as ProjectManager
        return Project.isValidProjectDirectory(path)
    }

    private func copyLogsToClipboard() {
        let allLogs = Logger.shared.logs.joined(separator: "\n")
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(allLogs, forType: .string)
        Logger.shared.log("Copied \(Logger.shared.logs.count) log entries to clipboard")
    }

    private func listAllWindows() {
        Logger.shared.log("=== Window Enumeration Debug ===")

        // Get all windows using CGWindowListCopyWindowInfo
        let windowList =
            CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]]
            ?? []

        Logger.shared.log("Found \(windowList.count) total windows")

        for (index, window) in windowList.enumerated() {
            let windowID = window[kCGWindowNumber as String] as? CGWindowID ?? 0
            let ownerName = window[kCGWindowOwnerName as String] as? String ?? "Unknown"
            let windowName = window[kCGWindowName as String] as? String ?? "No Title"
            let layer = window[kCGWindowLayer as String] as? Int ?? 0

            // Only log windows that have titles or are from common apps
            if !windowName.isEmpty || ["Visual Studio Code", "VSCode", "Code"].contains(ownerName) {
                Logger.shared.log(
                    "[\(index)] ID:\(windowID) Owner:\(ownerName) Title:\"\(windowName)\" Layer:\(layer)"
                )
            }
        }

        Logger.shared.log("=== End Window List ===")

        // Also copy to clipboard for easy inspection
        var output = "=== Window Enumeration Debug ===\n"
        output += "Found \(windowList.count) total windows\n\n"

        for (index, window) in windowList.enumerated() {
            let windowID = window[kCGWindowNumber as String] as? CGWindowID ?? 0
            let ownerName = window[kCGWindowOwnerName as String] as? String ?? "Unknown"
            let windowName = window[kCGWindowName as String] as? String ?? "No Title"
            let layer = window[kCGWindowLayer as String] as? Int ?? 0

            if !windowName.isEmpty || ["Visual Studio Code", "VSCode", "Code"].contains(ownerName) {
                output +=
                    "[\(index)] ID:\(windowID) Owner:\(ownerName) Title:\"\(windowName)\" Layer:\(layer)\n"
            }
        }

        output += "\n=== End Window List ==="

        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(output, forType: .string)

        Logger.shared.log("Window list copied to clipboard")
    }
}
