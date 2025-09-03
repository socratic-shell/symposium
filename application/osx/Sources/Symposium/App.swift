import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var agentManager = AgentManager()
    @StateObject private var settingsManager = SettingsManager()
    @StateObject private var permissionManager = PermissionManager()
    
    var body: some Scene {
        // Splash/Setup window - only shows when needed
        WindowGroup("splash") {
            SplashView()
                .environmentObject(agentManager)
                .environmentObject(settingsManager)
                .environmentObject(permissionManager)
                .onAppear {
                    Logger.shared.log("Splash window started")
                }
        }
        .windowResizability(.contentSize)
        .defaultAppStorage(.standard)
        
        // Project windows (can have multiple)
        WindowGroup("project", for: String.self) { $projectPath in
            if let projectPath = projectPath {
                ProjectWindow(projectPath: projectPath)
                    .environmentObject(agentManager)
                    .environmentObject(settingsManager)
                    .environmentObject(permissionManager)
            } else {
                Text("No project path provided")
                    .foregroundColor(.red)
            }
        }
        .windowResizability(.contentMinSize)
        .defaultAppStorage(.standard)
        .commands {
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
        let windowList = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] ?? []
        
        Logger.shared.log("Found \(windowList.count) total windows")
        
        for (index, window) in windowList.enumerated() {
            let windowID = window[kCGWindowNumber as String] as? CGWindowID ?? 0
            let ownerName = window[kCGWindowOwnerName as String] as? String ?? "Unknown"
            let windowName = window[kCGWindowName as String] as? String ?? "No Title"
            let layer = window[kCGWindowLayer as String] as? Int ?? 0
            
            // Only log windows that have titles or are from common apps
            if !windowName.isEmpty || ["Visual Studio Code", "VSCode", "Code"].contains(ownerName) {
                Logger.shared.log("[\(index)] ID:\(windowID) Owner:\(ownerName) Title:\"\(windowName)\" Layer:\(layer)")
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
                output += "[\(index)] ID:\(windowID) Owner:\(ownerName) Title:\"\(windowName)\" Layer:\(layer)\n"
            }
        }
        
        output += "\n=== End Window List ==="
        
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(output, forType: .string)
        
        Logger.shared.log("Window list copied to clipboard")
    }
}