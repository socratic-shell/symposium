import SwiftUI

struct ProjectWindow: View {
    let projectPath: String
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    
    var body: some View {
        ProjectWindowContent(
            projectPath: projectPath,
            agentManager: agentManager,
            settingsManager: settingsManager
        )
    }
}

private struct ProjectWindowContent: View {
    let projectPath: String
    let agentManager: AgentManager
    let settingsManager: SettingsManager
    
    @StateObject private var projectManager: ProjectManager
    
    init(projectPath: String, agentManager: AgentManager, settingsManager: SettingsManager) {
        self.projectPath = projectPath
        self.agentManager = agentManager
        self.settingsManager = settingsManager
        
        // Now we can properly initialize with the actual objects
        self._projectManager = StateObject(wrappedValue: ProjectManager(
            agentManager: agentManager,
            settingsManager: settingsManager,
            selectedAgent: settingsManager.selectedAgent
        ))
    }
    
    var body: some View {
        ProjectView(projectManager: projectManager)
            .onAppear {
                Logger.shared.log("ProjectWindow appeared for path: \(projectPath)")
                loadProject()
            }
    }
    private func loadProject() {
        Logger.shared.log("ProjectWindow: loadProject() called for path: \(projectPath)")
        do {
            try projectManager.openProject(at: projectPath)
            Logger.shared.log("ProjectWindow: Successfully loaded project at \(projectPath)")
        } catch {
            Logger.shared.log("ProjectWindow: Failed to load project at \(projectPath): \(error)")
        }
    }
}
