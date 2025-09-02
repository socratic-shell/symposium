import SwiftUI
import AppKit

struct ProjectView: View {
    @ObservedObject var projectManager: ProjectManager
    @ObservedObject var daemonManager: DaemonManager
    
    init(projectManager: ProjectManager) {
        self.projectManager = projectManager
        self.daemonManager = projectManager.mcpStatus
    }
    
    var body: some View {
        if let project = projectManager.currentProject {
            if daemonManager.isConnected {
                // Show full project interface when daemon is connected
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
                
                // IPC Daemon Status
                HStack(spacing: 4) {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                    
                    Text("Daemon Connected")
                        .font(.caption)
                        .foregroundColor(.green)
                }
                
                if let error = daemonManager.error {
                    Text("• \(error)")
                        .font(.caption)
                        .foregroundColor(.red)
                }
                
                Button("Close Project") {
                    projectManager.closeProject()
                }
                
                Button("New Taskspace") {
                    do {
                        try projectManager.createTaskspace()
                    } catch {
                        // TODO: Show error alert
                        print("Failed to create taskspace: \(error)")
                    }
                }
                .disabled(projectManager.isLoading)
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
                    
                    Text("Create a new taskspace to get started")
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                // Taskspace list
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
            } else {
                // Connecting to daemon
                VStack(spacing: 16) {
                    ProgressView()
                        .scaleEffect(1.2)
                    
                    Text("Connecting to daemon...")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    
                    Text("Project: \(project.name)")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .onAppear {
                    Logger.shared.log("ProjectView: Daemon connecting state appeared for project \(project.name)")
                }
            }
        } else if projectManager.isLoading {
            VStack {
                ProgressView()
                Text("Loading project...")
            }
        } else {
            Text("No project loaded")
                .foregroundColor(.red)
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
