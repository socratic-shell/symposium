import SwiftUI

struct SplashView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var appDelegate: AppDelegate
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss
    @State private var showingSettings = false

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
        
        // Notify AppDelegate about the active project for dock panel integration
        appDelegate.setCurrentProjectManager(projectManager, closeCallback: closeActiveProject)
        
        // Phase 22: Implement XOR invariant - hide splash window when project is active
        hideSplashWindow()
        
        // Phase 22: Show dock panel immediately when project opens
        showDockPanelImmediately(with: projectManager)
        
        Logger.shared.log("SplashView: Active project set, splash hidden, dock panel shown")
    }
    
    private func closeActiveProject() {
        Logger.shared.log("SplashView: Closing active project")
        
        // Clear the active project
        settingsManager.activeProjectPath = ""
        
        // Notify AppDelegate
        AppDelegate.shared?.setCurrentProjectManager(nil)
        
        // Phase 22: Implement XOR invariant - show splash window when no active project
        showSplashWindow()
        
        // TODO Phase 40: Close any VSCode windows associated with the project
        
        Logger.shared.log("SplashView: Active project closed, splash window shown")
    }
    
    // MARK: - Phase 22: XOR Invariant Helper Functions
    
    private func hideSplashWindow() {
        Logger.shared.log("SplashView: Dismissing splash window for XOR invariant")
        dismiss()
        Logger.shared.log("SplashView: Splash window dismissed")
    }
    
    private func showSplashWindow() {
        Logger.shared.log("SplashView: Opening splash window for XOR invariant")
        openWindow(id: "splash")
        Logger.shared.log("SplashView: Splash window opened")
    }
    
    private func showDockPanelImmediately(with projectManager: ProjectManager) {
        Logger.shared.log("SplashView: Showing dock panel immediately on project open")
        
        // Use AppDelegate to show the dock panel immediately
        let dockPosition = estimateDockPosition()
        appDelegate.showDockPanel(with: projectManager, at: dockPosition)
        Logger.shared.log("SplashView: Dock panel shown immediately at position: \(dockPosition)")
    }
    
    private func estimateDockPosition() -> NSPoint {
        // Reuse the same logic from AppDelegate for consistency
        guard let screen = NSScreen.main else {
            return NSPoint(x: 100, y: 100)
        }
        
        let screenFrame = screen.visibleFrame
        return NSPoint(
            x: screenFrame.midX,
            y: screenFrame.minY + 30  // Approximate dock height
        )
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
