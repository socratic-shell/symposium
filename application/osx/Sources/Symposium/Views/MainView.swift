import SwiftUI

struct MainView: View {
    @EnvironmentObject var permissionManager: PermissionManager
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
    }
}

