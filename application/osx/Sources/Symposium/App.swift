import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var daemonManager = DaemonManager()
    @StateObject private var agentManager = AgentManager()
    
    var body: some Scene {
        WindowGroup {
            MainView()
                .environmentObject(daemonManager)
                .environmentObject(agentManager)
        }
        
        Settings {
            SettingsView()
                .environmentObject(daemonManager)
                .environmentObject(agentManager)
        }
    }
}