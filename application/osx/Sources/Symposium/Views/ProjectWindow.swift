import SwiftUI
import AppKit

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
        .frame(minWidth: 300, idealWidth: calculateSidebarWidth(), minHeight: 400, idealHeight: calculateSidebarHeight())
        .navigationTitle(extractProjectName(from: projectPath))
        .onAppear {
            positionWindow()
        }
    }
    
    private func extractProjectName(from path: String) -> String {
        let url = URL(fileURLWithPath: path)
        let fileName = url.lastPathComponent
        
        // Remove .symposium extension if present
        if fileName.hasSuffix(".symposium") {
            return String(fileName.dropLast(10)) // Remove ".symposium"
        }
        return fileName
    }
    
    private func calculateSidebarWidth() -> CGFloat {
        guard let screen = NSScreen.main else { return 400 }
        let screenWidth = screen.frame.width
        
        // 1/3 of screen width, but cap at 500px for large screens, minimum 300px
        let targetWidth = screenWidth / 3
        return max(300, min(targetWidth, 500))
    }
    
    private func calculateSidebarHeight() -> CGFloat {
        guard let screen = NSScreen.main else { return 600 }
        let screenHeight = screen.visibleFrame.height
        
        // Use most of the screen height, leaving some margin
        return max(400, screenHeight - 150)
    }
    
    private func positionWindow() {
        // Position window on the left side of the screen
        DispatchQueue.main.async {
            if let window = NSApplication.shared.windows.last {
                guard let screen = NSScreen.main else { return }
                
                let screenFrame = screen.visibleFrame
                let windowWidth = calculateSidebarWidth()
                let windowHeight = calculateSidebarHeight()
                
                // Position on left side with some margin from edge
                let newFrame = NSRect(
                    x: screenFrame.minX + 20,
                    y: screenFrame.minY + 50,
                    width: windowWidth,
                    height: windowHeight
                )
                
                window.setFrame(newFrame, display: true)
            }
        }
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
