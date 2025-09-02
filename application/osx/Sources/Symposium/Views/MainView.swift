import SwiftUI

struct MainView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
    @State private var showingSettings = false
    @State private var projectManager: ProjectManager?
    
    var body: some View {
        VStack {
            // Simple header bar - Settings button only
            HStack {
                Spacer()
                
                Button("Settings") {
                    showingSettings = true
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            
            // Main content
            Group {
                if !permissionManager.hasAccessibilityPermission || !permissionManager.hasScreenRecordingPermission {
                    // Show settings if required permissions are missing
                    SettingsView()
                } else if agentManager.isScanning || (projectManager == nil && !settingsManager.lastProjectPath.isEmpty) {
                    // Show loading while scanning agents or validating remembered project
                    VStack(spacing: 16) {
                        ProgressView()
                            .scaleEffect(1.2)
                        
                        Text("Validating setup...")
                            .font(.headline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let projectManager = projectManager {
                    ProjectView(projectManager: projectManager)
                } else {
                    ProjectSelectionView(
                        onProjectCreated: { newProjectManager in
                            Logger.shared.log("MainView received onProjectCreated callback")
                            self.projectManager = newProjectManager
                        }
                    )
                }
            }
        }
        .frame(minWidth: 1000, idealWidth: 1200, maxWidth: .infinity,
               minHeight: 700, idealHeight: 800, maxHeight: .infinity)
        .sheet(isPresented: $showingSettings) {
            SettingsView()
        }
        .onAppear {
            permissionManager.checkAllPermissions()
        }
        .onChange(of: agentManager.isScanning) { isScanning in
            // When agent scanning completes, try to load remembered project
            if !isScanning && projectManager == nil {
                tryLoadRememberedProject()
            }
        }
    }
    
    private func tryLoadRememberedProject() {
        // Only try to load if we don't already have a project and there's a remembered path
        guard projectManager == nil, !settingsManager.lastProjectPath.isEmpty else {
            return
        }
        
        // Check if the remembered project path still exists and is valid
        guard FileManager.default.fileExists(atPath: settingsManager.lastProjectPath),
              Project.isValidProjectDirectory(settingsManager.lastProjectPath) else {
            Logger.shared.log("Remembered project path no longer valid, clearing: \(settingsManager.lastProjectPath)")
            settingsManager.lastProjectPath = ""
            return
        }
        
        // Try to load the remembered project
        let rememberedProjectManager = ProjectManager(
            agentManager: agentManager,
            settingsManager: settingsManager,
            selectedAgent: settingsManager.selectedAgent
        )
        
        do {
            try rememberedProjectManager.openProject(at: settingsManager.lastProjectPath)
            self.projectManager = rememberedProjectManager
            Logger.shared.log("Successfully loaded remembered project from: \(settingsManager.lastProjectPath)")
        } catch {
            Logger.shared.log("Failed to load remembered project: \(error)")
            // Clear the invalid path
            settingsManager.lastProjectPath = ""
        }
    }
}

