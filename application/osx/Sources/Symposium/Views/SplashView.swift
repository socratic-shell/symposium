import SwiftUI

struct SplashView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss
    @State private var showingSettings = false
    
    // Phase 20: Project lifecycle management
    @State private var activeProjectManager: ProjectManager?
    @State private var showingProject = false

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
            VStack {
                if !permissionManager.hasAccessibilityPermission
                    || !permissionManager.hasScreenRecordingPermission
                {
                    // Show settings if required permissions are missing
                    SettingsView()
                        .onAppear {
                            Logger.shared.log(
                                "SplashView: Showing SettingsView - missing permissions")
                        }
                } else if !agentManager.scanningCompleted && agentManager.scanningInProgress {
                    // Show loading while scanning agents
                    VStack(spacing: 16) {
                        ProgressView()
                            .scaleEffect(1.2)

                        Text("Scanning for agents...")
                            .font(.headline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .onAppear {
                        Logger.shared.log(
                            "SplashView: Showing agent scanning - scanningInProgress: \(agentManager.scanningInProgress)"
                        )
                    }
                } else if let activeProject = activeProjectManager?.currentProject {
                    // Phase 20: Show active project management UI
                    ActiveProjectView(
                        project: activeProject,
                        onCloseProject: {
                            Logger.shared.log("SplashView: Closing active project")
                            closeActiveProject()
                        }
                    )
                    .onAppear {
                        Logger.shared.log(
                            "SplashView: Showing ActiveProjectView for project: \(activeProject.name)"
                        )
                    }
                } else {
                    // Show project selection when no active project
                    ProjectSelectionView(
                        onProjectCreated: { projectManager in
                            Logger.shared.log("SplashView: Project created, setting as active")
                            setActiveProject(projectManager)
                        }
                    )
                    .onAppear {
                        Logger.shared.log(
                            "SplashView: Showing ProjectSelectionView - permissions OK, scanning done"
                        )
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .sheet(isPresented: $showingSettings) {
            SettingsView()
        }
        .onChange(of: agentManager.scanningCompleted) { completed in
            if completed {
                Logger.shared.log("SplashView: Agent scanning completed, checking for last project")
                checkForLastProject()
            }
        }
    }
    
    // MARK: - Phase 20: Project lifecycle management
    
    private func setActiveProject(_ projectManager: ProjectManager) {
        Logger.shared.log("SplashView: Setting active project manager")
        self.activeProjectManager = projectManager
        
        // Notify AppDelegate about the active project for dock panel integration
        AppDelegate.shared?.setCurrentProjectManager(projectManager)
        
        Logger.shared.log("SplashView: Active project set, dock panel integration ready")
    }
    
    private func closeActiveProject() {
        Logger.shared.log("SplashView: Closing active project")
        
        // Clear the active project
        self.activeProjectManager = nil
        settingsManager.activeProjectPath = ""
        
        // Notify AppDelegate
        AppDelegate.shared?.setCurrentProjectManager(nil)
        
        // TODO Phase 40: Close any VSCode windows associated with the project
        
        Logger.shared.log("SplashView: Active project closed, returning to project selection")
    }

    private func checkForLastProject() {
        Logger.shared.log(
            "SplashView: checkForLastProject - activeProjectPath: '\(settingsManager.activeProjectPath)'"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - hasAccessibility: \(permissionManager.hasAccessibilityPermission)"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - hasScreenRecording: \(permissionManager.hasScreenRecordingPermission)"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - agentsAvailable: \(agentManager.scanningCompleted)")

        // If we have a valid active project and permissions are OK, restore it
        if !settingsManager.activeProjectPath.isEmpty,
            permissionManager.hasAccessibilityPermission,
            permissionManager.hasScreenRecordingPermission,
            agentManager.scanningCompleted
        {

            Logger.shared.log(
                "SplashView: Found active project, restoring: \(settingsManager.activeProjectPath)"
            )
            
            // Phase 20: Restore the active project instead of opening a new window
            let restoredProjectManager = ProjectManager(
                agentManager: agentManager,
                settingsManager: settingsManager, 
                selectedAgent: settingsManager.selectedAgent,
                permissionManager: permissionManager
            )
            
            do {
                try restoredProjectManager.openProject(at: settingsManager.activeProjectPath)
                setActiveProject(restoredProjectManager)
                Logger.shared.log("SplashView: Successfully restored active project")
            } catch {
                Logger.shared.log("SplashView: Failed to restore active project: \(error)")
                // Clear invalid active project path
                settingsManager.activeProjectPath = ""
            }
        } else {
            Logger.shared.log("SplashView: No active project to restore - showing project selection")
        }
    }
}
