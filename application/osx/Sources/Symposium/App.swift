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
            ProjectSelectionView { projectManager in
                // When project is created/selected, close this window and open project window
                closeWindow(id: "choose-project")
                openProjectWindow(with: projectManager)
            }
            .environmentObject(agentManager)
            .environmentObject(settingsManager)
            .environmentObject(permissionManager)
            .onAppear {
                Logger.shared.log("Project selection window opened")
            }
            .onDisappear {
                Logger.shared.log("Project selection window closed")
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
                }
        }
        .windowResizability(.contentSize)
        
        // Main project window
        WindowGroup(id: "open-project") {
            if let projectManager = appDelegate.currentProjectManager {
                ProjectWindow(projectManager: projectManager)
                    .environmentObject(agentManager)
                    .environmentObject(settingsManager)
                    .environmentObject(permissionManager)
                    .onAppear {
                        Logger.shared.log("Project window opened")
                    }
                    .onDisappear {
                        Logger.shared.log("Project window closed")
                        // Clear current project manager when project window closes
                        appDelegate.currentProjectManager = nil
                    }
            } else {
                // Fallback if no project manager
                Text("No project loaded")
                    .frame(width: 400, height: 300)
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
                
                Divider()
                
                Button("Toggle Dock Panel") {
                    appDelegate.toggleDockPanel()
                }
                .keyboardShortcut("p", modifiers: [.command, .shift])
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
        openWindow(id: "open-project")
    }
    
    private func closeWindow(id: String) {
        Logger.shared.log("App: Closing window with id: \(id)")
        dismissWindow(id: id)
    }
    
    private func dismissSplash() {
        Logger.shared.log("App: Dismissing splash window")
        closeWindow(id: "splash")
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
        let hasPermissions = permissionManager.hasAccessibilityPermission && 
                           permissionManager.hasScreenRecordingPermission
        
        if !hasPermissions {
            Logger.shared.log("App: Missing permissions, showing settings window")
            dismissSplash()
            openWindow(id: "settings")
            return
        }
        
        // Check 2: Is there a previously opened project?
        if !settingsManager.activeProjectPath.isEmpty,
           isValidProject(at: settingsManager.activeProjectPath) {
            Logger.shared.log("App: Found valid last project at \(settingsManager.activeProjectPath), attempting to restore")
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
            Logger.shared.log("App: Successfully restored last project")
        } catch {
            Logger.shared.log("App: Failed to restore last project: \(error), showing project selection")
            // Clear invalid project path
            settingsManager.activeProjectPath = ""
            dismissSplash()
            openWindow(id: "choose-project")
        }
    }
    
    private func isValidProject(at path: String) -> Bool {
        // Check 1: Directory exists
        guard FileManager.default.fileExists(atPath: path) else {
            Logger.shared.log("App: Project path does not exist: \(path)")
            return false
        }
        
        // Check 2: Has .symposium directory
        let symposiumDir = "\(path)/.symposium"
        guard FileManager.default.fileExists(atPath: symposiumDir) else {
            Logger.shared.log("App: Missing .symposium directory: \(symposiumDir)")
            return false
        }
        
        // Check 3: Has valid project.json
        let projectFile = "\(symposiumDir)/project.json"
        guard FileManager.default.fileExists(atPath: projectFile) else {
            Logger.shared.log("App: Missing project.json: \(projectFile)")
            return false
        }
        
        // Check 4: Can parse project.json
        do {
            let data = try Data(contentsOf: URL(fileURLWithPath: projectFile))
            _ = try JSONDecoder().decode(Project.self, from: data)
            return true
        } catch {
            Logger.shared.log("App: Invalid project.json: \(error)")
            return false
        }
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
