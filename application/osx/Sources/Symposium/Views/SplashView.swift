import SwiftUI

struct SplashView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
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
                    ProjectSelectionView(
                        onProjectCreated: { projectManager in
                            Logger.shared.log("SplashView: Project created, opening project window")
                            Logger.shared.log(
                                "SplashView: ProjectManager currentProject: \(projectManager.currentProject?.name ?? "nil")"
                            )
                            if let projectPath = projectManager.currentProject?.directoryPath {
                                Logger.shared.log(
                                    "SplashView: Opening project window with path: \(projectPath)")
                                openWindow(id: "project", value: projectPath)
                                Logger.shared.log(
                                    "SplashView: Called openWindow, now dismissing splash")
                                dismiss()
                            } else {
                                Logger.shared.log("SplashView: ERROR - No project path available!")
                            }
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

    private func checkForLastProject() {
        Logger.shared.log(
            "SplashView: checkForLastProject - lastProjectPath: '\(settingsManager.lastProjectPath)'"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - hasAccessibility: \(permissionManager.hasAccessibilityPermission)"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - hasScreenRecording: \(permissionManager.hasScreenRecordingPermission)"
        )
        Logger.shared.log(
            "SplashView: checkForLastProject - agentsAvailable: \(agentManager.scanningCompleted)")

        // If we have a valid last project and permissions are OK, open it directly
        if !settingsManager.lastProjectPath.isEmpty,
            permissionManager.hasAccessibilityPermission,
            permissionManager.hasScreenRecordingPermission,
            agentManager.scanningCompleted
        {

            Logger.shared.log(
                "SplashView: Found last project, opening directly: \(settingsManager.lastProjectPath)"
            )
            openWindow(id: "project", value: settingsManager.lastProjectPath)
            dismiss()
        } else {
            Logger.shared.log("SplashView: Not opening last project - staying on splash")
        }
    }
}
