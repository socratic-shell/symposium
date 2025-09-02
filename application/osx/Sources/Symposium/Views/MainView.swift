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
                } else if let projectManager = projectManager, let project = projectManager.currentProject {
                    ProjectView(project: project, projectManager: projectManager)
                } else {
                    ProjectSelectionView(
                        onProjectCreated: { projectManager in
                            Logger.shared.log("MainView received onProjectCreated callback")
                            self.projectManager = projectManager
                            Logger.shared.log("MainView set projectManager, current project: \(projectManager.currentProject?.name ?? "nil")")
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

struct ProjectView: View {
    let project: Project
    @ObservedObject var projectManager: ProjectManager
    @State private var showingDebug = false
    
    private var daemonManager: DaemonManager {
        projectManager.mcpStatus
    }
    
    var body: some View {
        VStack {
            // Header with project info
            HStack {
                VStack(alignment: .leading) {
                    Text(project.name)
                        .font(.title)
                        .fontWeight(.bold)
                    
                    Text(project.gitURL)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                // MCP Status
                HStack(spacing: 4) {
                    Image(systemName: daemonManager.isConnected ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundColor(daemonManager.isConnected ? .green : .red)
                    
                    Text(daemonManager.isConnected ? "MCP Connected" : "MCP Disconnected")
                        .font(.caption)
                        .foregroundColor(daemonManager.isConnected ? .green : .red)
                }
                
                if let error = daemonManager.error {
                    Text("• \(error)")
                        .font(.caption)
                        .foregroundColor(.red)
                }
                
                if !daemonManager.debugOutput.isEmpty {
                    Button("Debug") {
                        showingDebug = true
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                }
                
                Button("Close Project") {
                    projectManager.closeProject()
                }
            }
            .padding()
            .background(Color.gray.opacity(0.1))
            
            // Main content area
            if project.taskspaces.isEmpty {
                // Empty state
                VStack(spacing: 16) {
                    Image(systemName: "tray")
                        .font(.system(size: 48))
                        .foregroundColor(.gray)
                    
                    Text("No taskspaces yet")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    
                    Text("Taskspaces will appear here when created by AI agents")
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                // Taskspace list (placeholder for now)
                ScrollView {
                    LazyVStack(spacing: 12) {
                        ForEach(project.taskspaces) { taskspace in
                            TaskspaceCard(taskspace: taskspace)
                        }
                    }
                    .padding()
                }
            }
        }
        .alert("MCP Debug Output", isPresented: $showingDebug) {
            Button("Copy") {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(daemonManager.debugOutput, forType: .string)
            }
            Button("OK") { }
        } message: {
            Text(daemonManager.debugOutput)
        }
    }
}

struct TaskspaceCard: View {
    let taskspace: Taskspace
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(taskspace.name)
                    .font(.headline)
                
                Spacer()
                
                if taskspace.needsAttention {
                    Image(systemName: "exclamationmark.circle.fill")
                        .foregroundColor(.orange)
                }
            }
            
            Text(taskspace.description)
                .font(.subheadline)
                .foregroundColor(.secondary)
            
            // Recent logs
            if !taskspace.logs.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(taskspace.logs.suffix(3)) { log in
                        HStack {
                            Text(log.category.icon)
                            Text(log.message)
                                .font(.caption)
                                .lineLimit(1)
                        }
                    }
                }
            }
        }
        .padding()
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(taskspace.needsAttention ? Color.orange : Color.clear, lineWidth: 2)
        )
    }
}
