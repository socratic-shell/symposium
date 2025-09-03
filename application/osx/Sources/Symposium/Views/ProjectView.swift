import SwiftUI
import AppKit

struct ProjectView: View {
    @ObservedObject var projectManager: ProjectManager
    @ObservedObject var ipcManager: IpcManager
    
    init(projectManager: ProjectManager) {
        self.projectManager = projectManager
        self.ipcManager = projectManager.mcpStatus
    }
    
    var body: some View {
        Group {
            if let project = projectManager.currentProject {
            if ipcManager.isConnected {
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
                
                if let error = ipcManager.error {
                    Text("• \(error)")
                        .font(.caption)
                        .foregroundColor(.red)
                }
                
                Button("Close Project") {
                    projectManager.closeProject()
                }
                
                Button(action: {
                    do {
                        try projectManager.createTaskspace()
                    } catch {
                        Logger.shared.log("Failed to create taskspace: \(error)")
                    }
                }) {
                    Image(systemName: "plus")
                }
                .help("New Taskspace")
                .disabled(projectManager.isLoading)
                
                Button(action: {
                    reregisterWindows()
                }) {
                    Image(systemName: "arrow.clockwise")
                }
                .help("Re-register Windows")
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
                            TaskspaceCard(taskspace: taskspace, projectManager: projectManager)
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
        .frame(minWidth: 300, idealWidth: 400, maxWidth: 500)
        .frame(minHeight: 400)
    }
    
    private func reregisterWindows() {
        guard let project = projectManager.currentProject else {
            Logger.shared.log("ProjectView: No current project for window re-registration")
            return
        }
        
        Logger.shared.log("ProjectView: Re-registering windows for \(project.taskspaces.count) taskspaces")
        
        for taskspace in project.taskspaces {
            // Send taskspace roll call message
            let payload = TaskspaceRollCallPayload(taskspaceUuid: taskspace.id.uuidString)
            ipcManager.sendBroadcastMessage(type: "taskspace_roll_call", payload: payload)
            Logger.shared.log("ProjectView: Sent roll call for taskspace: \(taskspace.name)")
        }
    }
}

struct TaskspaceCard: View {
    let taskspace: Taskspace
    let projectManager: ProjectManager
    @State private var showingDeleteConfirmation = false
    
    private var hasRegisteredWindow: Bool {
        projectManager.getWindow(for: taskspace.id) != nil
    }
    
    private var screenshotHeight: CGFloat {
        guard let screen = NSScreen.main else { return 120 }
        let screenAspectRatio = screen.frame.width / screen.frame.height
        let baseWidth: CGFloat = 200 // Approximate width available for screenshot
        return baseWidth / screenAspectRatio
    }
    
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
                
                Button(action: {
                    showingDeleteConfirmation = true
                }) {
                    Image(systemName: "trash")
                        .foregroundColor(.red)
                }
                .buttonStyle(.plain)
                .help("Delete taskspace")
            }
            
            Text(taskspace.description)
                .font(.subheadline)
                .foregroundColor(.secondary)
            
            // Screenshot placeholder
            RoundedRectangle(cornerRadius: 6)
                .fill(Color.gray.opacity(0.1))
                .frame(height: screenshotHeight)
                .overlay(
                    VStack(spacing: 4) {
                        Image(systemName: hasRegisteredWindow ? "display" : "arrow.clockwise")
                            .font(.title2)
                            .foregroundColor(.secondary)
                        Text(hasRegisteredWindow ? "VSCode" : "Disconnected")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                )
            
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
        .alert("Delete Taskspace", isPresented: $showingDeleteConfirmation) {
            Button("Cancel", role: .cancel) { }
            Button("Delete", role: .destructive) {
                do {
                    try projectManager.deleteTaskspace(taskspace)
                } catch {
                    Logger.shared.log("Failed to delete taskspace: \(error)")
                }
            }
        } message: {
            Text("Are you sure you want to delete '\(taskspace.name)'? This will permanently remove all files and cannot be undone.")
        }
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(taskspace.needsAttention ? Color.orange : Color.clear, lineWidth: 2)
        )
    }
}
