import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var daemonManager = DaemonManager()
    @StateObject private var agentManager = AgentManager()
    @AppStorage("selectedAgent") private var selectedAgent: String = "qcli"
    
    var body: some Scene {
        WindowGroup {
            MainView()
                .environmentObject(daemonManager)
                .environmentObject(agentManager)
                .onAppear {
                    startClientIfNeeded()
                }
        }
        
        Settings {
            SettingsView()
                .environmentObject(daemonManager)
                .environmentObject(agentManager)
        }
    }
    
    private func startClientIfNeeded() {
        agentManager.scanForAgents()
        
        // Wait a moment for agent scan to complete, then start client
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            if let selectedAgentInfo = agentManager.availableAgents.first(where: { $0.id == selectedAgent }),
               selectedAgentInfo.isInstalled && selectedAgentInfo.isMCPConfigured,
               let mcpPath = selectedAgentInfo.mcpServerPath {
                daemonManager.startClient(mcpServerPath: mcpPath)
            }
        }
    }
}