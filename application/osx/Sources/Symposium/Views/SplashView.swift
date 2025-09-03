import SwiftUI

struct SplashView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
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
                            Logger.shared.log("SplashView: Project created, should open project window")
                            // TODO: Open project window and close splash
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
        }
    }
}
