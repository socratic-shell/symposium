import AppKit
import SwiftUI

struct ProjectView: View {
    @ObservedObject var projectManager: ProjectManager
    @ObservedObject var ipcManager: IpcManager
    
    // Phase 22: Optional callback for closing the project from dock panel
    var onCloseProject: (() -> Void)?
    
    // Step 7: Optional callback for just dismissing the panel
    var onDismiss: (() -> Void)?
    
    // Step 5: Expand/collapse state management
    @State private var expandedTaskspace: UUID? = nil
    
    // Task description dialog state
    @State private var showingNewTaskspaceDialog = false {
        didSet {
            Logger.shared.log("ProjectView: showingNewTaskspaceDialog changed to \(showingNewTaskspaceDialog)")
        }
    }
    
    // Stacked windows state
    @State private var stackedWindowsEnabled = false

    init(projectManager: ProjectManager, onCloseProject: (() -> Void)? = nil, onDismiss: (() -> Void)? = nil) {
        self.projectManager = projectManager
        self.ipcManager = projectManager.mcpStatus
        self.onCloseProject = onCloseProject
        self.onDismiss = onDismiss
    }
    
    // Step 7: Smart dismissal helper
    private func dismissPanel() {
        onDismiss?()
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
                            
                            // Stacked Windows Toggle
                            Toggle("Stack Windows", isOn: $stackedWindowsEnabled)
                                .font(.caption)
                                .help("When enabled, clicking a taskspace positions all windows at the same location")
                                .onChange(of: stackedWindowsEnabled) { newValue in
                                    projectManager.setStackedWindowsEnabled(newValue)
                                    Logger.shared.log("ProjectView: Stacked windows \(newValue ? "enabled" : "disabled")")
                                }

                            Button(action: {
                                Logger.shared.log("ProjectView: + button clicked, showing dialog")
                                showingNewTaskspaceDialog = true
                            }) {
                                Image(systemName: "plus")
                            }
                            .help("New Taskspace")
                            .disabled(projectManager.isLoading)
                            .popover(isPresented: $showingNewTaskspaceDialog) {
                                NewTaskspaceDialog(projectManager: projectManager)
                            }

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
                        if let expandedTaskspace = expandedTaskspace {
                            // Detail mode - show expanded taskspace
                            expandedTaskspaceView(for: expandedTaskspace)
                        } else if project.taskspaces.isEmpty {
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
                            // Grid mode - show all taskspaces
                            taskspaceGridView
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
        .frame(minHeight: 400)
        .onAppear {
            // Initialize stacked windows state from project
            if let project = projectManager.currentProject {
                stackedWindowsEnabled = project.stackedWindowsEnabled
                Logger.shared.log("ProjectView: Initialized stacked windows state: \(stackedWindowsEnabled) for project \(project.name)")
            }
        }
    }
    
    // MARK: - Step 5: Helper Views
    
    private var taskspaceGridView: some View {
        GeometryReader { geometry in
            let taskspaceWidth = calculateTaskspaceWidth()
            let columns = calculateGridColumns(panelWidth: geometry.size.width, taskspaceWidth: taskspaceWidth)
            
            ScrollView {
                LazyVGrid(columns: Array(repeating: GridItem(.fixed(taskspaceWidth)), count: columns), spacing: 16) {
                    ForEach(projectManager.currentProject?.taskspaces ?? []) { taskspace in
                        TaskspaceCard(
                            taskspace: taskspace, 
                            projectManager: projectManager,
                            onExpand: { expandedTaskspace = taskspace.id },
                            onDismiss: dismissPanel
                        )
                    }
                }
                .padding()
            }
        }
    }
    
    // MARK: - Step 6: Grid Layout Helpers
    
    private func calculateTaskspaceWidth() -> CGFloat {
        let screenshotWidth: CGFloat = 120
        
        // Measure sample Star Trek log message
        let sampleText = "Captain, we're getting mysterious sensor readings"
        let textAttributes = [NSAttributedString.Key.font: NSFont.systemFont(ofSize: 13)]
        let sampleTextWidth = sampleText.size(withAttributes: textAttributes).width
        
        let padding: CGFloat = 40 // Internal card padding
        return screenshotWidth + sampleTextWidth + padding
    }
    
    private func calculateGridColumns(panelWidth: CGFloat, taskspaceWidth: CGFloat) -> Int {
        let availableWidth = panelWidth - 32 // Account for padding
        let maxColumns = Int(floor(availableWidth / taskspaceWidth))
        return max(1, maxColumns) // Always at least 1 column
    }
    
    private func expandedTaskspaceView(for taskspaceId: UUID) -> some View {
        VStack(spacing: 0) {
            // Breadcrumb header
            HStack {
                Button(action: { expandedTaskspace = nil }) {
                    HStack(spacing: 4) {
                        Image(systemName: "arrow.left")
                        Text("Back to Grid")
                    }
                    .foregroundColor(.blue)
                }
                .buttonStyle(.plain)
                
                Image(systemName: "chevron.right")
                    .foregroundColor(.secondary)
                    .font(.caption)
                
                if let taskspace = projectManager.currentProject?.taskspaces.first(where: { $0.id == taskspaceId }) {
                    Text(taskspace.name)
                        .font(.headline)
                        .fontWeight(.semibold)
                } else {
                    Text("Unknown Taskspace")
                        .font(.headline)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
            }
            .padding()
            .background(Color.gray.opacity(0.1))
            
            // Expanded taskspace content
            if let taskspace = projectManager.currentProject?.taskspaces.first(where: { $0.id == taskspaceId }) {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        // Taskspace header with screenshot and info
                        HStack(alignment: .top, spacing: 16) {
                            // Screenshot
                            Group {
                                if let screenshot = projectManager.getScreenshot(for: taskspace.id) {
                                    Image(nsImage: screenshot)
                                        .resizable()
                                        .aspectRatio(contentMode: .fill)
                                        .frame(width: 120, height: 80)
                                        .cornerRadius(6)
                                        .clipped()
                                } else {
                                    RoundedRectangle(cornerRadius: 6)
                                        .fill(Color.gray.opacity(0.1))
                                        .frame(width: 120, height: 80)
                                        .overlay(
                                            Text("No Preview")
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                        )
                                }
                            }
                            
                            // Info column
                            VStack(alignment: .leading, spacing: 8) {
                                Text(taskspace.description)
                                    .font(.subheadline)
                                    .foregroundColor(.secondary)
                                
                                HStack {
                                    Button("Focus Window") {
                                        // TODO: Focus taskspace window
                                    }
                                    .disabled(projectManager.getWindow(for: taskspace.id) == nil)
                                    
                                    Button("Settings") {
                                        // TODO: Show taskspace settings
                                    }
                                }
                                .buttonStyle(.borderless)
                            }
                            
                            Spacer()
                        }
                        
                        Divider()
                        
                        // Full log list
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Activity Log")
                                .font(.headline)
                                .fontWeight(.semibold)
                            
                            if taskspace.logs.isEmpty {
                                Text("No activity yet")
                                    .foregroundColor(.secondary)
                                    .italic()
                            } else {
                                ForEach(taskspace.logs) { log in
                                    HStack(spacing: 8) {
                                        Text(log.category.icon)
                                            .font(.system(size: 12))
                                        
                                        Text(log.message)
                                            .font(.system(size: 12))
                                            .foregroundColor(.secondary)
                                        
                                        Spacer()
                                        
                                        Text(log.timestamp, format: .dateTime.hour().minute().second())
                                            .font(.system(size: 10))
                                            .foregroundColor(.secondary)
                                            .opacity(0.7)
                                    }
                                    .padding(.vertical, 2)
                                }
                            }
                        }
                        
                        Spacer(minLength: 0)
                    }
                    .padding()
                }
            } else {
                VStack {
                    Text("Taskspace not found")
                        .foregroundColor(.secondary)
                        .italic()
                    Spacer()
                }
                .padding()
            }
        }
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
    @State private var deleteBranch = false
    @State private var branchName = ""
    
    // Step 5: Callback for expand functionality
    var onExpand: (() -> Void)? = nil
    
    // Step 7: Callback for panel dismissal on VSCode engagement
    var onDismiss: (() -> Void)? = nil

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
            // Phase 40: Focus existing active window
            Logger.shared.log("TaskspaceCard: Focusing active taskspace: \(taskspace.name)")
            let success = projectManager.focusTaskspaceWindow(for: taskspace)
            if !success {
                Logger.shared.log("TaskspaceCard: Focus failed, taskspace may have become dormant")
            }
        } else {
            // Phase 30: Activate dormant taskspace by launching VSCode
            Logger.shared.log("TaskspaceCard: Activating dormant taskspace: \(taskspace.name)")
            projectManager.launchVSCode(for: taskspace)
        }
        
        // Step 7: Dismiss panel after VSCode engagement
        onDismiss?()
    }

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
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
            VStack(alignment: .leading, spacing: 8) {
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
                        onExpand?()
                    }) {
                        Image(systemName: "arrow.up.left.and.arrow.down.right")
                            .foregroundColor(.blue)
                    }
                    .buttonStyle(.plain)
                    .help("View details")

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
                    .lineLimit(3)

                // Recent logs (expanded)
                if !taskspace.logs.isEmpty {
                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text("Recent Activity")
                                .font(.caption)
                                .fontWeight(.medium)
                                .foregroundColor(.secondary)
                            
                            Spacer()
                            
                            if taskspace.logs.count > 10 {
                                Button(action: {
                                    // TODO: Implement full log viewer
                                    Logger.shared.log("TaskspaceCard: TODO - Show full logs for: \(taskspace.name)")
                                }) {
                                    Text("View All (\(taskspace.logs.count))")
                                        .font(.system(size: 9))
                                        .foregroundColor(.blue)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                        
                        ForEach(taskspace.logs.suffix(10)) { log in
                            HStack(spacing: 4) {
                                Text(log.category.icon)
                                    .font(.system(size: 10))
                                Text(log.message)
                                    .font(.system(size: 10))
                                    .foregroundColor(.secondary)
                                    .lineLimit(2)
                            }
                        }
                    }
                }
                
                Spacer(minLength: 0)
            }
        }
        .padding(16)
        .background(Color.gray.opacity(0.05))
        .cornerRadius(8)
        .onTapGesture {
            handleTaskspaceClick()
        }
        .sheet(isPresented: $showingDeleteConfirmation) {
            DeleteTaskspaceDialog(
                taskspaceName: taskspace.name,
                branchName: branchName,
                deleteBranch: $deleteBranch,
                onConfirm: {
                    do {
                        try projectManager.deleteTaskspace(taskspace, deleteBranch: deleteBranch)
                    } catch {
                        Logger.shared.log("Failed to delete taskspace: \(error)")
                    }
                    showingDeleteConfirmation = false
                },
                onCancel: {
                    showingDeleteConfirmation = false
                }
            )
        }
        .onAppear {
            branchName = projectManager.getBranchName(for: taskspace)
        }
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(taskspace.needsAttention ? Color.orange : Color.clear, lineWidth: 2)
        )
    }
}

struct DeleteTaskspaceDialog: View {
    let taskspaceName: String
    let branchName: String
    @Binding var deleteBranch: Bool
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Delete Taskspace")
                .font(.headline)
            
            Text("Are you sure you want to delete '\(taskspaceName)'? This will permanently remove all files and cannot be undone.")
                .multilineTextAlignment(.center)
            
            if !branchName.isEmpty {
                HStack {
                    Toggle("Also delete the branch `\(branchName)` from git", isOn: $deleteBranch)
                    Spacer()
                }
            }
            
            HStack {
                Button("Cancel") {
                    onCancel()
                }
                .keyboardShortcut(.escape)
                
                Spacer()
                
                Button("Delete") {
                    onConfirm()
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.return)
            }
        }
        .padding()
        .frame(width: 400)
    }
}

struct NewTaskspaceDialog: View {
    @ObservedObject var projectManager: ProjectManager
    @Environment(\.dismiss) private var dismiss
    
    @State private var taskDescription = ""
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("New Taskspace")
                .font(.headline)
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Describe what you want to accomplish:")
                    .font(.subheadline)
                
                TextEditor(text: $taskDescription)
                    .frame(minHeight: 100)
                    .overlay(
                        RoundedRectangle(cornerRadius: 4)
                            .stroke(Color.gray.opacity(0.3), lineWidth: 1)
                    )
            }
            
            HStack {
                Button("Cancel") {
                    Logger.shared.log("NewTaskspaceDialog: Cancel clicked")
                    dismiss()
                }
                .keyboardShortcut(.escape)
                
                Spacer()
                
                Button("Create") {
                    createTaskspace()
                }
                .disabled(taskDescription.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.return)
            }
        }
        .padding()
        .frame(width: 500, height: 250)
        .onAppear {
            Logger.shared.log("NewTaskspaceDialog: Dialog appeared")
        }
    }
    
    private func createTaskspace() {
        let trimmedDescription = taskDescription.trimmingCharacters(in: .whitespacesAndNewlines)
        
        let initialPrompt = """
        Hi, welcome! You are a new agent just getting started as part of the project \(projectManager.currentProject?.name ?? ""). \
        This is a taskspace, a separate copy of the project's files where you can work undisturbed. \
        The user's description of the task to be done follows after this message. \
        Can you start by reading the description and using the 'update_taskspace' tool to provide a better name/description for the taskspace? \
        Before doing any work on the task, be sure to ask the user clarifying questions to better understand their intent.

        User's task description:
        \(trimmedDescription)
        """
        
        do {
            try projectManager.createTaskspace(
                name: "New Task",
                description: "Getting started...",
                initialPrompt: initialPrompt
            )
            dismiss()
        } catch {
            Logger.shared.log("Failed to create taskspace: \(error)")
        }
    }
}
