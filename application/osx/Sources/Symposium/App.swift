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
                    // TODO: Implement appStart() state machine logic
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
        // Note: SwiftUI doesn't provide direct window closing, but the window will close
        // when the user dismisses it or when we open another window
    }
    
    private func showProjectSelectionWindow() {
        Logger.shared.log("App: Opening project selection window")
        openWindow(id: "choose-project")
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
