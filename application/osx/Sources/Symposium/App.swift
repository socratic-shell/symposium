import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var agentManager = AgentManager()
    
    var body: some Scene {
        WindowGroup {
            MainView()
                .environmentObject(agentManager)
        }
        
        Settings {
            SettingsView()
                .environmentObject(agentManager)
        }
    }
}