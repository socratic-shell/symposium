import AppKit
import SwiftUI

struct ProjectView: View {
    @EnvironmentObject var appDelegate: AppDelegate
    
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

    // Step 7: Smart dismissal helper
    private func dismissPanel() {
        onDismiss?()
    }

    var body: some View {
        Group {
            if let projectManager = appDelegate.currentProjectManager,
               let project = projectManager.currentProject {
                if projectManager.mcpStatus.isConnected {
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

                            if let error = projectManager.mcpStatus.error {
                                Text("• \(error)")
                                    .font(.caption)
                                    .foregroundColor(.red)
                            }
                            
                            // Stacked Windows Toggle
                            Toggle("Stack Windows", isOn: $stackedWindowsEnabled)
                                .font(.caption)
                                .help("When enabled, clicking a taskspace positions all windows at the same location")
                                .onChange(of: stackedWindowsEnabled) { _, newValue in
                                    if let projectManager = appDelegate.currentProjectManager {
                                        projectManager.setStackedWindowsEnabled(newValue)
                                        Logger.shared.log("ProjectView: Stacked windows \(newValue ? "enabled" : "disabled")")
                                    }
                                }

                            Button(action: {
                                Logger.shared.log("ProjectView: + button clicked, showing dialog")
                                showingNewTaskspaceDialog = true
                            }) {
                                Image(systemName: "plus")
                            }
                            .help("New Taskspace")
                            .disabled(projectManager.isLoading)
                            .onHover { hovering in
                                if !projectManager.isLoading {
                                    NSCursor.pointingHand.set()
                                }
                            }
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
                            .onHover { hovering in
                                if !projectManager.isLoading {
                                    NSCursor.pointingHand.set()
                                }
                            }
                            
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
                                .onHover { hovering in
                                    NSCursor.pointingHand.set()
                                }
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
            } else if let projectManager = appDelegate.currentProjectManager, projectManager.isLoading {
                VStack {
                    ProgressView()
                    Text("Loading project...")
                }
            } else {
                Text("No project selected")
                    .foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minHeight: 400)
        .onAppear {
            // Initialize stacked windows state from project
            if let projectManager = appDelegate.currentProjectManager,
               let project = projectManager.currentProject {
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
                    if let projectManager = appDelegate.currentProjectManager {
                        ForEach(projectManager.currentProject?.taskspaces ?? []) { taskspace in
                            TaskspaceCard(
                                taskspace: taskspace, 
                                projectManager: projectManager,
                                onExpand: { expandedTaskspace = taskspace.id },
                                onDismiss: dismissPanel
                            )
                        }
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
                .onHover { hovering in
                    NSCursor.pointingHand.set()
                }
                
                Image(systemName: "chevron.right")
                    .foregroundColor(.secondary)
                    .font(.caption)
                
                if let projectManager = appDelegate.currentProjectManager,
                   let taskspace = projectManager.currentProject?.taskspaces.first(where: { $0.id == taskspaceId }) {
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
            if let projectManager = appDelegate.currentProjectManager,
               let taskspace = projectManager.currentProject?.taskspaces.first(where: { $0.id == taskspaceId }) {
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
        guard let projectManager = appDelegate.currentProjectManager,
              let project = projectManager.currentProject else {
            Logger.shared.log("ProjectView: No current project for window re-registration")
            return
        }

        Logger.shared.log(
            "ProjectView: Re-registering windows for \(project.taskspaces.count) taskspaces")

        for taskspace in project.taskspaces {
            // Send taskspace roll call message
            let payload = TaskspaceRollCallPayload(taskspaceUuid: taskspace.id.uuidString)
            projectManager.mcpStatus.sendBroadcastMessage(type: "taskspace_roll_call", payload: payload)
            Logger.shared.log("ProjectView: Sent roll call for taskspace: \(taskspace.name)")
        }
    }
}

struct TaskspaceCard: View {
    let taskspace: Taskspace
    @ObservedObject var projectManager: ProjectManager
    @State private var showingDeleteConfirmation = false
    @State private var deleteBranch = false
    @State private var cachedBranchInfo: (branchName: String, isMerged: Bool, unmergedCommits: Int, hasUncommittedChanges: Bool) = ("", false, 0, false)
    @State private var isHovered = false
    @State private var isPressed = false
    
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
        // Always update activation time and reorder on any taskspace click
        if var project = projectManager.currentProject {
            // Acknowledge attention signals when user clicks on the taskspace
            if let taskspaceIndex = project.findTaskspaceIndex(uuid: taskspace.id.uuidString) {
                project.taskspaces[taskspaceIndex].acknowledgeAttentionSignals()
                
                // Save the taskspace to persist the acknowledged signals
                do {
                    try project.taskspaces[taskspaceIndex].save(in: project.directoryPath)
                    Logger.shared.log("TaskspaceCard: Acknowledged attention signals for \(taskspace.name)")
                } catch {
                    Logger.shared.log("TaskspaceCard: Failed to save acknowledged signals: \(error)")
                }
            }
            
            project.activateTaskspace(uuid: taskspace.id.uuidString)
            projectManager.currentProject = project
            
            // Save the updated project to persist the new ordering
            do {
                try project.save()
                Logger.shared.log("TaskspaceCard: Updated activation order for \(taskspace.name)")
            } catch {
                Logger.shared.log("TaskspaceCard: Failed to save activation order: \(error)")
            }
        }
        
        if hasRegisteredWindow {
            // Phase 40: Focus existing active window (no need to reorder again)
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
                    .onHover { hovering in
                        NSCursor.pointingHand.set()
                    }

                    Button(action: {
                        showingDeleteConfirmation = true
                    }) {
                        Image(systemName: "trash")
                            .foregroundColor(.red)
                    }
                    .buttonStyle(.plain)
                    .help("Delete taskspace")
                    .onHover { hovering in
                        NSCursor.pointingHand.set()
                    }
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
        .background(
            Color.gray.opacity(isPressed ? 0.8 : (isHovered ? 0.15 : 0.05))
                .animation(.easeInOut(duration: isPressed ? 0.1 : 0.2), value: isHovered)
                .animation(.easeInOut(duration: 0.1), value: isPressed)
        )
        .cornerRadius(8)
        .onHover { hovering in
            isHovered = hovering
        }
        .onTapGesture {
            // Flash effect
            isPressed = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                isPressed = false
            }
            handleTaskspaceClick()
        }
        .sheet(isPresented: $showingDeleteConfirmation) {
            DeleteTaskspaceDialog(
                taskspaceName: taskspace.name,
                taskspace: taskspace,
                projectManager: projectManager,
                deleteBranch: $deleteBranch,
                onConfirm: {
                    Task {
                        do {
                            try await projectManager.deleteTaskspace(taskspace, deleteBranch: deleteBranch)
                        } catch {
                            Logger.shared.log("Failed to delete taskspace: \(error)")
                        }
                        await MainActor.run {
                            showingDeleteConfirmation = false
                        }
                    }
                },
                onCancel: {
                    // Send cancellation response for pending deletion request
                    projectManager.sendDeletionCancelledResponse(for: taskspace.id)
                    showingDeleteConfirmation = false
                }
            )
        }
        .onChange(of: taskspace.pendingDeletion) { pending in
            if pending {
                showingDeleteConfirmation = true
                // Clear the flag after showing dialog
                if var updatedProject = projectManager.currentProject,
                   let taskspaceIndex = updatedProject.taskspaces.firstIndex(where: { $0.id == taskspace.id }) {
                    updatedProject.taskspaces[taskspaceIndex].pendingDeletion = false
                    projectManager.currentProject = updatedProject
                }
            }
        }
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(taskspace.needsAttention ? Color.orange : Color.clear, lineWidth: 2)
        )
    }
}

struct DeleteTaskspaceDialog: View {
    let taskspaceName: String
    let taskspace: Taskspace
    let projectManager: ProjectManager
    @Binding var deleteBranch: Bool
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    @State private var cachedBranchInfo: (branchName: String, isMerged: Bool, unmergedCommits: Int, hasUncommittedChanges: Bool) = ("", false, 0, false)
    @State private var isLoadingBranchInfo = true
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Delete Taskspace")
                .font(.headline)
            
            Text("Are you sure you want to delete '\(taskspaceName)'? This will permanently remove all files and cannot be undone.")
                .multilineTextAlignment(.center)
            
            if isLoadingBranchInfo {
                HStack {
                    ProgressView()
                        .scaleEffect(0.8)
                    Text("Checking branch status...")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else if !cachedBranchInfo.branchName.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Toggle("Also delete the branch `\(cachedBranchInfo.branchName)` from git", isOn: $deleteBranch)
                        Spacer()
                    }
                    
                    if cachedBranchInfo.unmergedCommits > 0 || cachedBranchInfo.hasUncommittedChanges {
                        VStack(alignment: .leading, spacing: 4) {
                            if cachedBranchInfo.unmergedCommits > 0 {
                                HStack {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundColor(.orange)
                                    Text("\(cachedBranchInfo.unmergedCommits) commit\(cachedBranchInfo.unmergedCommits == 1 ? "" : "s") from this branch do not appear in the main branch.")
                                        .font(.caption)
                                        .foregroundColor(.orange)
                                }
                                .padding(.leading, 20)
                            }
                            
                            if cachedBranchInfo.hasUncommittedChanges {
                                HStack {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundColor(.orange)
                                    Text("This taskspace contains uncommitted changes.")
                                        .font(.caption)
                                        .foregroundColor(.orange)
                                }
                                .padding(.leading, 20)
                            }
                            
                            if cachedBranchInfo.unmergedCommits > 0 || cachedBranchInfo.hasUncommittedChanges {
                                HStack {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundColor(.orange)
                                    Text("Are you sure you want to delete the taskspace?")
                                        .font(.caption)
                                        .foregroundColor(.orange)
                                        .fontWeight(.medium)
                                }
                                .padding(.leading, 20)
                            }
                        }
                    } else {
                        HStack {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                            Text("This branch is safe to delete (no unmerged commits or uncommitted changes)")
                                .font(.caption)
                                .foregroundColor(.green)
                        }
                        .padding(.leading, 20)
                    }
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
        .onAppear {
            Task {
                let manager = projectManager
                let ts = taskspace
                cachedBranchInfo = await Task.detached {
                    manager.getTaskspaceBranchInfo(for: ts)
                }.value
                
                isLoadingBranchInfo = false
                
                // Set default deleteBranch toggle based on safety analysis
                deleteBranch = (cachedBranchInfo.unmergedCommits == 0 && !cachedBranchInfo.hasUncommittedChanges)
            }
        }
        .padding()
        .frame(width: 400)
    }
}

struct NewTaskspaceDialog: View {
    @ObservedObject var projectManager: ProjectManager
    @Environment(\.dismiss) private var dismiss
    
    @AppStorage("newTaskspaceDialogText") private var taskDescription = ""
    @State private var selectedCollaborator = "sparkle"
    
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
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Collaboration Style:")
                    .font(.subheadline)
                
                Picker("Collaborator", selection: $selectedCollaborator) {
                    Text("Sparkle").tag("sparkle")
                    Text("Socrates").tag("socrates") 
                    Text("None").tag("base-agent")
                }
                .pickerStyle(.segmented)
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
        .frame(width: 500, height: 300)
        .onAppear {
            Logger.shared.log("NewTaskspaceDialog: Dialog appeared")
        }
    }
    
    private func createTaskspace() {
        let trimmedDescription = taskDescription.trimmingCharacters(in: .whitespacesAndNewlines)
        
        do {
            try projectManager.createTaskspace(
                name: "New Task",
                description: "Getting started...",
                initialPrompt: projectManager.generateInitialPrompt(taskDescription: trimmedDescription),
                collaborator: selectedCollaborator
            )
            taskDescription = "" // Clear persisted text after successful creation
            dismiss()
        } catch {
            Logger.shared.log("Failed to create taskspace: \(error)")
        }
    }
}
