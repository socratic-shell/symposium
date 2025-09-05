import AppKit
import SwiftUI

struct ProjectView: View {
    @ObservedObject var projectManager: ProjectManager
    @ObservedObject var ipcManager: IpcManager
    
    // Phase 22: Optional callback for closing the project from dock panel
    var onCloseProject: (() -> Void)?

    init(projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil) {
        self.projectManager = projectManager
        self.ipcManager = projectManager.mcpStatus
        self.onCloseProject = onCloseProject
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
                            
                            // Phase 22: Close Project button (only show if callback provided)
                            if let onClose = onCloseProject {
                                Button(action: {
                                    Logger.shared.log("ProjectView: Close Project button pressed")
                                    onClose()
                                }) {
                                    Image(systemName: "xmark.circle")
                                }
                                .help("Close Project")
                                .foregroundColor(.red)
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

                                Text("Create a new taskspace to get started")
                                    .foregroundColor(.secondary)
                            }
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                        } else {
                            // Taskspace list
                            ScrollView {
                                LazyVStack(spacing: 12) {
                                    ForEach(project.taskspaces) { taskspace in
                                        TaskspaceCard(
                                            taskspace: taskspace, projectManager: projectManager)
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
                        Logger.shared.log(
                            "ProjectView: Daemon connecting state appeared for project \(project.name)"
                        )
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

        Logger.shared.log(
            "ProjectView: Re-registering windows for \(project.taskspaces.count) taskspaces")

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
    @ObservedObject var projectManager: ProjectManager
    @State private var showingDeleteConfirmation = false

    private var hasRegisteredWindow: Bool {
        projectManager.getWindow(for: taskspace.id) != nil
    }
    
    // Phase 30: Two-dimensional state helpers
    private var isHatchling: Bool {
        switch taskspace.state {
        case .hatchling: return true
        case .resume: return false
        }
    }
    
    private var stateIcon: String {
        if hasRegisteredWindow {
            return isHatchling ? "hourglass" : "display"
        } else {
            return isHatchling ? "play.circle" : "arrow.clockwise"
        }
    }
    
    private var stateText: String {
        if hasRegisteredWindow {
            return isHatchling ? "Starting..." : "Connected"
        } else {
            return isHatchling ? "Click to start" : "Click to connect"
        }
    }

    
    private func handleTaskspaceClick() {
        if hasRegisteredWindow {
            // TODO Phase 40: Focus existing window
            Logger.shared.log("TaskspaceCard: TODO - Focus active taskspace: \(taskspace.name)")
        } else {
            // Phase 30: Activate dormant taskspace by launching VSCode
            Logger.shared.log("TaskspaceCard: Activating dormant taskspace: \(taskspace.name)")
            projectManager.launchVSCode(for: taskspace)
        }
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            // Left: Screenshot thumbnail
            Group {
                if let screenshot = projectManager.getScreenshot(for: taskspace.id) {
                    // Show screenshot - live if active, heavily greyed with overlay if dormant
                    ZStack {
                        Image(nsImage: screenshot)
                            .resizable()
                            .aspectRatio(contentMode: .fill)
                            .frame(width: 120, height: 80)
                            .cornerRadius(6)
                            .clipped()
                            .opacity(hasRegisteredWindow ? 1.0 : 0.3)
                            .saturation(hasRegisteredWindow ? 1.0 : 0.2)
                        
                        // Overlay action text for dormant screenshots
                        if !hasRegisteredWindow {
                            RoundedRectangle(cornerRadius: 6)
                                .fill(Color.black.opacity(0.4))
                                .frame(width: 120, height: 80)
                                .overlay(
                                    VStack(spacing: 2) {
                                        Image(systemName: stateIcon)
                                            .font(.caption)
                                            .foregroundColor(.white)
                                        Text(stateText)
                                            .font(.system(size: 8))
                                            .foregroundColor(.white)
                                            .fontWeight(.medium)
                                    }
                                )
                        }
                    }
                } else {
                    // Show placeholder
                    RoundedRectangle(cornerRadius: 6)
                        .fill(Color.gray.opacity(0.1))
                        .frame(width: 120, height: 80)
                        .overlay(
                            VStack(spacing: 2) {
                                Image(systemName: stateIcon)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                Text(stateText)
                                    .font(.system(size: 8))
                                    .foregroundColor(.secondary)
                            }
                        )
                }
            }
            
            // Right: Content column
            VStack(alignment: .leading, spacing: 6) {
                // Header row
                HStack {
                    Text(taskspace.name)
                        .font(.headline)
                        .fontWeight(.semibold)

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

                // Description
                Text(taskspace.description)
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .lineLimit(2)

                // Recent logs (compact)
                if !taskspace.logs.isEmpty {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(taskspace.logs.suffix(2)) { log in
                            HStack(spacing: 4) {
                                Text(log.category.icon)
                                    .font(.system(size: 10))
                                Text(log.message)
                                    .font(.system(size: 10))
                                    .foregroundColor(.secondary)
                                    .lineLimit(1)
                            }
                        }
                    }
                }
                
                Spacer(minLength: 0)
            }
        }
        .padding()
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
        .onTapGesture {
            handleTaskspaceClick()
        }
        .alert("Delete Taskspace", isPresented: $showingDeleteConfirmation) {
            Button("Cancel", role: .cancel) {}
            Button("Delete", role: .destructive) {
                do {
                    try projectManager.deleteTaskspace(taskspace)
                } catch {
                    Logger.shared.log("Failed to delete taskspace: \(error)")
                }
            }
        } message: {
            Text(
                "Are you sure you want to delete '\(taskspace.name)'? This will permanently remove all files and cannot be undone."
            )
        }
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(taskspace.needsAttention ? Color.orange : Color.clear, lineWidth: 2)
        )
    }
}
