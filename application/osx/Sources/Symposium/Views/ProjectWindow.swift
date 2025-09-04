import SwiftUI

struct ProjectWindow: View {
    let projectPath: String
    @StateObject private var projectManager: ProjectManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    
    init(projectPath: String) {
        self.projectPath = projectPath
        // Initialize ProjectManager for this specific project
        let agentManager = AgentManager()
        let settingsManager = SettingsManager()
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
                Logger.shared.log("ProjectWindow: About to call loadProject()")
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
