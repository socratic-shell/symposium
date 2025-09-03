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
            Group {
                if !permissionManager.hasAccessibilityPermission || !permissionManager.hasScreenRecordingPermission {
                    // Show settings if required permissions are missing
                    SettingsView()
                } else if agentManager.isScanning {
                    // Show loading while scanning agents
                    VStack(spacing: 16) {
                        ProgressView()
                            .scaleEffect(1.2)
                        
                        Text("Scanning for agents...")
                            .font(.headline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    ProjectSelectionView(
                        onProjectCreated: { projectManager in
                            Logger.shared.log("SplashView: Project created, opening project window")
                            if let projectPath = projectManager.currentProject?.directoryPath {
                                openWindow(id: "project", value: projectPath)
                                dismiss()
                            }
                        }
                    )
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .sheet(isPresented: $showingSettings) {
            SettingsView()
        }
        .onAppear {
            Logger.shared.log("SplashView appeared")
            checkForLastProject()
        }
    }
    
    private func checkForLastProject() {
        // If we have a valid last project and permissions are OK, open it directly
        if !settingsManager.lastProjectPath.isEmpty,
           permissionManager.hasAccessibilityPermission,
           permissionManager.hasScreenRecordingPermission,
           !agentManager.isScanning {
            
            Logger.shared.log("SplashView: Found last project, opening directly: \(settingsManager.lastProjectPath)")
            openWindow(id: "project", value: settingsManager.lastProjectPath)
            dismiss()
        }
    }
}
