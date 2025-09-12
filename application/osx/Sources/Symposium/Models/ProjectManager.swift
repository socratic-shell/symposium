import AppKit
import CoreGraphics
import Foundation
import Combine

/// Manages project creation, loading, and operations
class ProjectManager: ObservableObject, IpcMessageDelegate {
    @Published var currentProject: Project?
    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var taskspaceScreenshots: [UUID: NSImage] = [:]

    private let ipcManager = IpcManager()
    private let agentManager: AgentManager
    private let settingsManager: SettingsManager
    private let selectedAgent: AgentType
    private let permissionManager: PermissionManager

    private static var nextInstanceId = 1
    private let instanceId: Int
    private var cancellables = Set<AnyCancellable>()

    // Window associations for current project
    @Published private var taskspaceWindows: [UUID: CGWindowID] = [:]

    // Window stacking state
    private var stackTracker: WindowStackTracker?

    // Public access to settings manager for UI
    var settings: SettingsManager {
        return settingsManager
    }

    /// Update stacked windows setting for current project
    func setStackedWindowsEnabled(_ enabled: Bool) {
        guard var project = currentProject else { return }
        project.stackedWindowsEnabled = enabled
        currentProject = project

        // Stop tracking if disabling stacked windows
        if !enabled {
            stackTracker?.stopTracking()
        }

        // Save the updated project
        do {
            try project.save()
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Updated stacked windows setting to \(enabled) for project \(project.name)"
            )
        } catch {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to save stacked windows setting: \(error)")
        }
    }

    // Window close detection timer
    private var windowCloseTimer: Timer?

    var mcpStatus: IpcManager { ipcManager }

    /// Get screenshot for a taskspace (returns cached version from @Published property)
    func getScreenshot(for taskspaceId: UUID) -> NSImage? {
        print(
            "DEBUG: Requesting screenshot for \(taskspaceId), have screenshots for: \(taskspaceScreenshots.keys)"
        )
        return taskspaceScreenshots[taskspaceId]
    }

    // Screenshot manager (macOS 14.0+ assumed)
    private lazy var screenshotManager: ScreenshotManager = {
        ScreenshotManager(permissionManager: permissionManager)
    }()

    init(
        agentManager: AgentManager, settingsManager: SettingsManager, selectedAgent: AgentType,
        permissionManager: PermissionManager
    ) {
        self.instanceId = ProjectManager.nextInstanceId
        ProjectManager.nextInstanceId += 1

        self.agentManager = agentManager
        self.settingsManager = settingsManager
        self.selectedAgent = selectedAgent
        self.permissionManager = permissionManager

        Logger.shared.log("ProjectManager[\(instanceId)]: Created")
        // ScreenshotManager initialization is deferred via lazy var
        
        // Subscribe to IpcManager changes to republish them
        ipcManager.objectWillChange
            .sink { [weak self] in
                self?.objectWillChange.send()
            }
            .store(in: &cancellables)
    }

    deinit {
        Logger.shared.log("ProjectManager[\(instanceId)]: Cleaning up")
        stopWindowCloseDetection()
    }

    /// Open an existing Symposium project
    func openProject(at directoryPath: String) throws {
        isLoading = true
        defer { isLoading = false }

        // Validate project directory
        guard Project.isValidProjectDirectory(directoryPath) else {
            throw ProjectError.invalidProjectDirectory
        }

        // Load project
        let project = try Project.load(from: directoryPath)

        // Load existing taskspaces
        var loadedProject = project
        loadedProject.taskspaces = try loadTaskspaces(from: directoryPath)

        // Set as current project
        setCurrentProject(loadedProject)
    }

    /// Helper to set current project and register as IPC delegate
    private func setCurrentProject(_ project: Project) {
        // Stop window detection for previous project
        stopWindowCloseDetection()

        // Clear previous project state
        taskspaceWindows.removeAll()

        self.currentProject = project
        self.errorMessage = nil

        // Save project path for next app launch
        self.settingsManager.activeProjectPath = project.directoryPath

        // Register as IPC delegate for this project
        self.ipcManager.addDelegate(self)
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Registered as IPC delegate for project: \(project.name)"
        )

        // Phase 30: Do NOT auto-launch VSCode - taskspaces start dormant until user activates them
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Project opened with \(project.taskspaces.count) dormant taskspaces"
        )

        // Load existing screenshots from disk for visual persistence
        self.loadExistingScreenshots()

        // Start automatic window close detection
        self.startWindowCloseDetection()

        self.startMCPClient()
    }

    /// Launch VSCode for a specific taskspace (used for user-activated awakening)
    func launchVSCode(for taskspace: Taskspace) {
        guard let project = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot launch VSCode - no current project")
            return
        }

        launchVSCode(for: taskspace, in: project.directoryPath)
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: User-activated VSCode for taskspace: \(taskspace.name)")
    }

    // MARK: - Legacy method (no longer auto-launches on project open)
    // /// Launch VSCode for all active taskspaces (both hatchling and resume states)
    // private func launchVSCodeForActiveTaskspaces(in project: Project) {
    //     let activeTaskspaces = project.taskspaces.filter { taskspace in
    //         switch taskspace.state {
    //         case .hatchling, .resume:
    //             return true
    //         }
    //     }
    //
    //     for taskspace in activeTaskspaces {
    //         launchVSCode(for: taskspace, in: project.directoryPath)
    //     }
    //
    //     if !activeTaskspaces.isEmpty {
    //         Logger.shared.log(
    //             "ProjectManager: Launched VSCode for \(activeTaskspaces.count) active taskspaces")
    //     }
    // }

    /// Load all taskspaces from project directory
    private func loadTaskspaces(from projectPath: String) throws -> [Taskspace] {
        let fileManager = FileManager.default
        var taskspaces: [Taskspace] = []

        // Find all task-* directories
        let contents = try fileManager.contentsOfDirectory(atPath: projectPath)
        let taskDirectories = contents.filter { $0.hasPrefix("task-") }

        for taskDir in taskDirectories {
            let taskspacePath = "\(projectPath)/\(taskDir)/taskspace.json"
            if fileManager.fileExists(atPath: taskspacePath) {
                do {
                    let taskspace = try Taskspace.load(from: taskspacePath)
                    taskspaces.append(taskspace)
                } catch {
                    // Log error but continue loading other taskspaces
                    print("Failed to load taskspace at \(taskspacePath): \(error)")
                }
            }
        }

        return taskspaces.sorted { $0.createdAt > $1.createdAt }
    }

    /// Close current project
    func closeProject() {
        // Stop window stack tracking
        stackTracker?.stopTracking()
        stackTracker = nil

        // Unregister as IPC delegate
        ipcManager.removeDelegate(self)
        Logger.shared.log("ProjectManager[\(instanceId)]: Unregistered as IPC delegate")

        stopMCPClient()
        DispatchQueue.main.async {
            self.currentProject = nil
            self.errorMessage = nil
        }
    }

    private func startMCPClient() {
        Logger.shared.log("ProjectManager[\(instanceId)]: Starting daemon client")
        // Stop any existing client first
        ipcManager.stopClient()

        // Start client if we have a valid selected agent
        if let selectedAgentInfo = agentManager.availableAgents.first(where: {
            $0.type == selectedAgent
        }) {
            Logger.shared.log(
                "ProjectManager: Found agent \(selectedAgent): installed=\(selectedAgentInfo.isInstalled), mcpConfigured=\(selectedAgentInfo.isMCPConfigured)"
            )

            if selectedAgentInfo.isInstalled && selectedAgentInfo.isMCPConfigured,
                let mcpPath = selectedAgentInfo.mcpServerPath
            {
                Logger.shared.log(
                    "ProjectManager[\(instanceId)]: Starting daemon with path: \(mcpPath)")
                ipcManager.startClient(mcpServerPath: mcpPath)
            } else {
                Logger.shared.log(
                    "ProjectManager: Agent not ready - installed: \(selectedAgentInfo.isInstalled), mcpConfigured: \(selectedAgentInfo.isMCPConfigured), mcpPath: \(selectedAgentInfo.mcpServerPath ?? "nil")"
                )
            }
        } else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: No agent found with id: \(selectedAgent)")
            Logger.shared.log(
                "ProjectManager: Available agents: \(agentManager.availableAgents.map { $0.id })")
        }
    }

    private func stopMCPClient() {
        ipcManager.stopClient()
    }

    /// Set error message
    func setError(_ message: String) {
        DispatchQueue.main.async {
            self.errorMessage = message
        }
    }

    /// Clear error message
    func clearError() {
        DispatchQueue.main.async {
            self.errorMessage = nil
        }
    }

    /// Delete a taskspace and its directory
    func deleteTaskspace(_ taskspace: Taskspace, deleteBranch: Bool = false) throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        isLoading = true
        defer { isLoading = false }

        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)

        // Get current branch name before removing worktree
        let branchName = getTaskspaceBranch(for: taskspaceDir)

        // Remove git worktree (this also removes the directory)
        let worktreeProcess = Process()
        worktreeProcess.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        worktreeProcess.arguments = ["worktree", "remove", taskspaceDir, "--force"]
        worktreeProcess.currentDirectoryURL = URL(fileURLWithPath: project.directoryPath)

        try worktreeProcess.run()
        worktreeProcess.waitUntilExit()

        if worktreeProcess.terminationStatus != 0 {
            Logger.shared.log(
                "Warning: Failed to remove git worktree, falling back to directory removal")
            // Fallback: remove directory if worktree removal failed
            try FileManager.default.removeItem(atPath: taskspaceDir)
        }

        // Optionally delete the branch
        if deleteBranch && !branchName.isEmpty {
            let branchProcess = Process()
            branchProcess.executableURL = URL(fileURLWithPath: "/usr/bin/git")
            branchProcess.arguments = ["branch", "-D", branchName]
            branchProcess.currentDirectoryURL = URL(fileURLWithPath: project.directoryPath)

            try branchProcess.run()
            branchProcess.waitUntilExit()

            if branchProcess.terminationStatus != 0 {
                Logger.shared.log("Warning: Failed to delete branch \(branchName)")
            } else {
                Logger.shared.log("ProjectManager[\(instanceId)]: Deleted branch \(branchName)")
            }
        }

        // Remove from current project
        DispatchQueue.main.async {
            var updatedProject = project
            updatedProject.taskspaces.removeAll { $0.id == taskspace.id }
            self.currentProject = updatedProject
            Logger.shared.log(
                "ProjectManager[\(self.instanceId)]: Deleted taskspace \(taskspace.name)")
        }
    }

    func getBranchName(for taskspace: Taskspace) -> String {
        guard let project = currentProject else {
            return ""
        }

        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        return getTaskspaceBranch(for: taskspaceDir)
    }

    private func getTaskspaceBranch(for taskspaceDir: String) -> String {
        guard let project = currentProject else {
            return ""
        }
        
        let repoName = extractRepoName(from: project.gitURL)
        let worktreeDir = "\(taskspaceDir)/\(repoName)"
        
        do {
            return try getCurrentBranch(in: worktreeDir)
        } catch {
            Logger.shared.log("Failed to get branch name for taskspace dir \(taskspaceDir): \(error)")
            return ""
        }
    }

    func getTaskspaceBranchInfo(for taskspace: Taskspace) -> (branchName: String, isMerged: Bool, unmergedCommits: Int, hasUncommittedChanges: Bool) {
        guard let project = currentProject else {
            return ("", false, 0, false)
        }

        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        let branchName = getTaskspaceBranch(for: taskspaceDir)
        
        if branchName.isEmpty {
            return ("", false, 0, false)
        }

        let repoName = extractRepoName(from: project.gitURL)
        let worktreeDir = "\(taskspaceDir)/\(repoName)"
        
        do {
            let baseBranch = try getBaseBranch(for: project)
            let isMerged = try isBranchMerged(branchName: branchName, baseBranch: baseBranch, in: worktreeDir)
            let unmergedCommits = try getUnmergedCommitCount(branchName: branchName, baseBranch: baseBranch, in: worktreeDir)
            let hasUncommittedChanges = try hasUncommittedChanges(in: worktreeDir)
            
            return (branchName, isMerged, unmergedCommits, hasUncommittedChanges)
        } catch {
            Logger.shared.log("Failed to get branch info for taskspace \(taskspace.name): \(error)")
            return (branchName, false, 0, false)
        }
    }

    private func isBranchMerged(branchName: String, baseBranch: String, in directory: String) throws -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["merge-base", "--is-ancestor", branchName, baseBranch]
        process.currentDirectoryURL = URL(fileURLWithPath: directory)

        try process.run()
        process.waitUntilExit()

        return process.terminationStatus == 0
    }

    private func getUnmergedCommitCount(branchName: String, baseBranch: String, in directory: String) throws -> Int {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["rev-list", "--count", "\(branchName)", "--not", baseBranch]
        process.currentDirectoryURL = URL(fileURLWithPath: directory)

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            return 0
        }

        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "0"
        return Int(output) ?? 0
    }
    
    private func hasUncommittedChanges(in directory: String) throws -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["status", "--porcelain"]
        process.currentDirectoryURL = URL(fileURLWithPath: directory)

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            return false
        }

        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return !output.isEmpty
    }

    private func getCurrentBranch(in directory: String) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["branch", "--show-current"]
        process.currentDirectoryURL = URL(fileURLWithPath: directory)

        let pipe = Pipe()
        process.standardOutput = pipe

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            return ""
        }

        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        return String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
            ?? ""
    }

    /// Create a new taskspace with default values
    /// Generate comprehensive initial prompt for new taskspaces
    func generateInitialPrompt(taskDescription: String) -> String {
        return """
            Hi, welcome! You are a new agent just getting started as part of the project \(currentProject?.name ?? ""). \
            This is a taskspace, a separate copy of the project's files where you can work undisturbed. \
            The user's description of the task to be done follows after this message. \
            Can you start by reading the description and using the 'update_taskspace' tool to provide a better name/description for the taskspace? \
            Before doing any work on the task, be sure to ask the user clarifying questions to better understand their intent.

            User's task description:
            \(taskDescription)
            """
    }

    func createTaskspace() throws {
        try createTaskspace(
            name: "Unnamed taskspace",
            description: "TBD",
            initialPrompt:
                "This is a newly created taskspace. Figure out what the user wants to do and update the name/description appropriately using the `update_taskspace` tool."
        )
    }

    /// Create a new taskspace with specified values
    func createTaskspace(name: String, description: String, initialPrompt: String) throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        isLoading = true
        defer { isLoading = false }

        // Create taskspace with provided values
        let taskspace = Taskspace(
            name: name,
            description: description,
            initialPrompt: initialPrompt
        )

        // Create taskspace directory
        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        try FileManager.default.createDirectory(
            atPath: taskspaceDir,
            withIntermediateDirectories: true,
            attributes: nil
        )

        // Ensure bare repository exists (create if this is the first taskspace)
        if !bareRepositoryExists(in: project.directoryPath) {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Creating bare repository for first taskspace")
            let bareProcess = Process()
            bareProcess.executableURL = URL(fileURLWithPath: "/usr/bin/git")
            bareProcess.arguments = [
                "clone", "--bare", project.gitURL, "\(project.directoryPath)/.git",
            ]

            try bareProcess.run()
            bareProcess.waitUntilExit()

            if bareProcess.terminationStatus != 0 {
                throw ProjectError.gitCloneFailed
            }
        } else {
            Logger.shared.log("ProjectManager[\(instanceId)]: Bare repository already exists")
        }

        // Create worktree for this taskspace with unique branch
        let branchName = "taskspace-\(taskspace.id.uuidString)"
        let repoName = extractRepoName(from: project.gitURL)
        let worktreeDir = "\(taskspaceDir)/\(repoName)"

        // Determine the base branch to start from
        let baseBranch = try getBaseBranch(for: project)
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Creating worktree with branch \(branchName) from \(baseBranch)"
        )

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["worktree", "add", worktreeDir, "-b", branchName, baseBranch]
        process.currentDirectoryURL = URL(fileURLWithPath: project.directoryPath)

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            throw ProjectError.gitCloneFailed
        }

        // Save taskspace metadata
        try taskspace.save(in: project.directoryPath)

        // Phase 30: Do NOT auto-launch VSCode - new taskspaces start dormant until user clicks
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Created new taskspace '\(taskspace.name)' (dormant until activated)"
        )

        // Add to current project
        DispatchQueue.main.async {
            var updatedProject = project
            updatedProject.taskspaces.append(taskspace)
            self.currentProject = updatedProject
        }
    }

    /// Extract repository name from git URL
    private func extractRepoName(from gitURL: String) -> String {
        let url = gitURL.replacingOccurrences(of: ".git", with: "")
        return URL(string: url)?.lastPathComponent ?? "repo"
    }

    /// Get the base branch for new taskspaces (from project.defaultBranch or auto-detect)
    private func getBaseBranch(for project: Project) throws -> String {
        // If project specifies a default branch, use it
        if let defaultBranch = project.defaultBranch, !defaultBranch.isEmpty {
            return defaultBranch
        }

        // Auto-detect origin's default branch
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["symbolic-ref", "refs/remotes/origin/HEAD"]
        process.currentDirectoryURL = URL(fileURLWithPath: project.directoryPath)

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus == 0 {
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8)?.trimmingCharacters(
                in: .whitespacesAndNewlines)
            {
                // Output is like "refs/remotes/origin/main", extract "origin/main"
                if output.hasPrefix("refs/remotes/") {
                    return String(output.dropFirst("refs/remotes/".count))
                }
            }
        }

        // Fallback to origin/main
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Could not detect origin's default branch, falling back to origin/main"
        )
        return "origin/main"
    }

    /// Check if a bare git repository exists at the project path
    private func bareRepositoryExists(in projectPath: String) -> Bool {
        let gitPath = "\(projectPath)/.git"
        let fileManager = FileManager.default

        // Check if .git exists
        guard fileManager.fileExists(atPath: gitPath) else {
            return false
        }

        // Check if it's a bare repository by looking for the 'bare' config
        let configPath = "\(gitPath)/config"
        guard let configContent = try? String(contentsOfFile: configPath) else {
            return false
        }

        return configContent.contains("bare = true")
    }
}

/// Errors that can occur during project operations
enum ProjectError: LocalizedError {
    case directoryAlreadyExists
    case invalidProjectDirectory
    case failedToCreateDirectory
    case failedToSaveProject
    case noCurrentProject
    case gitCloneFailed

    var errorDescription: String? {
        switch self {
        case .directoryAlreadyExists:
            return "A project with this name already exists in the selected directory"
        case .invalidProjectDirectory:
            return "The selected directory is not a valid Symposium project"
        case .failedToCreateDirectory:
            return "Failed to create project directory"
        case .failedToSaveProject:
            return "Failed to save project metadata"
        case .noCurrentProject:
            return "No project is currently loaded"
        case .gitCloneFailed:
            return "Failed to clone git repository"
        }
    }
}

// MARK: - Window Management

extension ProjectManager {
    /// Associate a window with a taskspace
    func associateWindow(_ windowID: CGWindowID, with taskspaceUuid: String) -> Bool {
        guard let uuid = UUID(uuidString: taskspaceUuid) else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Invalid UUID format: \(taskspaceUuid)")
            return false
        }

        // Verify taskspace exists in current project
        guard let project = currentProject,
            project.findTaskspace(uuid: taskspaceUuid) != nil
        else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Taskspace not found for UUID: \(taskspaceUuid)")
            return false
        }

        taskspaceWindows[uuid] = windowID
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Associated window \(windowID) with taskspace \(uuid)")

        // If stacked windows is enabled, position the new window to match existing stack
        if let project = currentProject, project.stackedWindowsEnabled {
            positionNewWindowInStack(windowID: windowID, taskspaceId: uuid)
        }

        // Capture screenshot when window is first registered
        Logger.shared.log(
            "ProjectManager: Attempting screenshot capture for window \(windowID), taskspace \(uuid)"
        )
        Task { @MainActor in
            Logger.shared.log("ProjectManager[\(instanceId)]: Starting screenshot capture task")
            await captureAndCacheScreenshot(windowId: windowID, for: uuid)
        }

        return true
    }

    /// Get window ID for a taskspace
    func getWindow(for taskspaceUuid: UUID) -> CGWindowID? {
        return taskspaceWindows[taskspaceUuid]
    }

    /// Focus an active taskspace's VSCode window
    func focusTaskspaceWindow(for taskspace: Taskspace) -> Bool {
        guard let windowID = taskspaceWindows[taskspace.id] else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot focus taskspace \(taskspace.name) - no registered window"
            )
            return false
        }

        // Verify window still exists before trying to focus it
        guard isWindowStillOpen(windowID: windowID) else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot focus taskspace \(taskspace.name) - window no longer exists"
            )
            // Clean up stale window reference
            taskspaceWindows.removeValue(forKey: taskspace.id)
            return false
        }

        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Focusing window \(windowID) for taskspace: \(taskspace.name)"
        )

        // Check if stacked windows mode is enabled for this project
        if let project = currentProject, project.stackedWindowsEnabled {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Stacked windows mode enabled - positioning all taskspace windows"
            )
            return focusWindowWithStacking(targetTaskspace: taskspace, targetWindowID: windowID)
        } else {
            // Use Core Graphics to focus the window normally
            let result = focusWindow(windowID: windowID)

            if result {
                Logger.shared.log(
                    "ProjectManager[\(instanceId)]: Successfully focused window for taskspace: \(taskspace.name)"
                )
            } else {
                Logger.shared.log(
                    "ProjectManager[\(instanceId)]: Failed to focus window for taskspace: \(taskspace.name)"
                )
            }

            return result
        }
    }

    /// Focus a window by its CGWindowID using Core Graphics APIs
    private func focusWindow(windowID: CGWindowID) -> Bool {
        // Get window info to find the owning process
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard
            let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID)
                as? [[String: Any]]
        else {
            return false
        }

        guard
            let windowInfo = windowList.first(where: { window in
                if let id = window[kCGWindowNumber as String] as? CGWindowID {
                    return id == windowID
                }
                return false
            })
        else {
            return false
        }

        // Get the process ID that owns this window
        guard let ownerPID = windowInfo[kCGWindowOwnerPID as String] as? pid_t else {
            return false
        }

        // Get the running application for this process
        guard let app = NSRunningApplication(processIdentifier: ownerPID) else {
            return false
        }

        // Activate the application (brings it to front)
        let success = app.activate()

        if success {
            // Small delay to let the app activation complete
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                // Try to bring the specific window to front using Accessibility APIs
                self.focusWindowViaAccessibility(windowID: windowID, processID: ownerPID)
            }
        }

        return success
    }

    /// Focus a window with stacking - positions all other taskspace windows at the same location
    private func focusWindowWithStacking(targetTaskspace: Taskspace, targetWindowID: CGWindowID)
        -> Bool
    {
        // First, focus the target window normally
        let focusResult = focusWindow(windowID: targetWindowID)

        if !focusResult {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to focus target window for stacking")
            return false
        }

        // Get the position and size of the target window
        guard let targetWindowInfo = getWindowInfo(for: targetWindowID) else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Could not get target window info for stacking")
            return focusResult
        }

        let targetBounds = targetWindowInfo.bounds
        Logger.shared.log("ProjectManager[\(instanceId)]: Target window bounds: \(targetBounds)")

        // Position all other taskspace windows at the same location (but behind)
        guard let project = currentProject else { return focusResult }

        var followerWindowIDs: [CGWindowID] = []

        for taskspace in project.taskspaces {
            // Skip the target taskspace
            if taskspace.id == targetTaskspace.id { continue }

            // Skip taskspaces without registered windows
            guard let windowID = taskspaceWindows[taskspace.id] else { continue }

            // Verify window still exists
            guard isWindowStillOpen(windowID: windowID) else {
                taskspaceWindows.removeValue(forKey: taskspace.id)
                continue
            }

            // Position this window at the same location as the target
            positionWindow(windowID: windowID, to: targetBounds)
            followerWindowIDs.append(windowID)
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Positioned window for taskspace \(taskspace.name) in stack"
            )
        }

        // Set up window stack tracking for drag and resize following
        if stackTracker == nil {
            stackTracker = WindowStackTracker()
        }
        var allWindowIDs = [targetWindowID]
        allWindowIDs.append(contentsOf: followerWindowIDs)
        stackTracker?.startTracking(windowIDs: allWindowIDs)

        return focusResult
    }
    
    /// Position a newly registered window to match existing stacked windows
    private func positionNewWindowInStack(windowID: CGWindowID, taskspaceId: UUID) {
        guard currentProject != nil else { return }
        
        // Find any existing window to use as reference for positioning
        var referenceWindowID: CGWindowID?
        for (id, existingWindowID) in taskspaceWindows {
            if id != taskspaceId && isWindowStillOpen(windowID: existingWindowID) {
                referenceWindowID = existingWindowID
                break
            }
        }
        
        guard let refWindowID = referenceWindowID,
              let refWindowInfo = getWindowInfo(for: refWindowID) else {
            Logger.shared.log("ProjectManager[\(instanceId)]: No reference window found for stacking new window")
            return
        }
        
        // Position the new window to match the reference window
        positionWindow(windowID: windowID, to: refWindowInfo.bounds)
        Logger.shared.log("ProjectManager[\(instanceId)]: Positioned new window \(windowID) to match stack")
        
        // Update the stack tracker to include the new window
        if stackTracker != nil {
            var allWindowIDs: [CGWindowID] = []
            for (_, existingWindowID) in taskspaceWindows {
                if isWindowStillOpen(windowID: existingWindowID) {
                    allWindowIDs.append(existingWindowID)
                }
            }
            stackTracker?.startTracking(windowIDs: allWindowIDs)
            Logger.shared.log("ProjectManager[\(instanceId)]: Updated stack tracker with new window")
        }
    }

    /// Get window information including bounds
    private func getWindowInfo(for windowID: CGWindowID) -> (bounds: CGRect, processID: pid_t)? {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard
            let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID)
                as? [[String: Any]]
        else {
            return nil
        }

        guard
            let windowInfo = windowList.first(where: { window in
                if let id = window[kCGWindowNumber as String] as? CGWindowID {
                    return id == windowID
                }
                return false
            })
        else {
            return nil
        }

        guard let boundsDict = windowInfo[kCGWindowBounds as String] as? [String: Any],
            let x = boundsDict["X"] as? CGFloat,
            let y = boundsDict["Y"] as? CGFloat,
            let width = boundsDict["Width"] as? CGFloat,
            let height = boundsDict["Height"] as? CGFloat,
            let processID = windowInfo[kCGWindowOwnerPID as String] as? pid_t
        else {
            return nil
        }

        let bounds = CGRect(x: x, y: y, width: width, height: height)
        return (bounds: bounds, processID: processID)
    }

    /// Use Accessibility APIs to focus a specific window within an application
    private func focusWindowViaAccessibility(windowID: CGWindowID, processID: pid_t) {
        let app = AXUIElementCreateApplication(processID)

        var windowsRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(
            app, kAXWindowsAttribute as CFString, &windowsRef)

        guard result == .success,
            let windows = windowsRef as? [AXUIElement]
        else {
            return
        }

        // Find the window with matching CGWindowID
        for window in windows {
            if let axWindowID = getWindowID(from: window), axWindowID == windowID {
                // Focus this specific window
                AXUIElementPerformAction(window, kAXRaiseAction as CFString)
                break
            }
        }
    }

    /// Position a window to specific bounds using Accessibility APIs
    private func positionWindow(windowID: CGWindowID, to bounds: CGRect) {
        guard let windowInfo = getWindowInfo(for: windowID) else { return }

        let app = AXUIElementCreateApplication(windowInfo.processID)

        var windowsRef: CFTypeRef?
        let result = AXUIElementCopyAttributeValue(
            app, kAXWindowsAttribute as CFString, &windowsRef)

        guard result == .success,
            let windows = windowsRef as? [AXUIElement]
        else {
            return
        }

        // Find the window with matching CGWindowID
        for window in windows {
            if let axWindowID = getWindowID(from: window), axWindowID == windowID {
                // Set position
                var position = CGPoint(x: bounds.origin.x, y: bounds.origin.y)
                let positionValue = AXValueCreate(AXValueType.cgPoint, &position)
                AXUIElementSetAttributeValue(
                    window, kAXPositionAttribute as CFString, positionValue!)

                // Set size
                var size = CGSize(width: bounds.size.width, height: bounds.size.height)
                let sizeValue = AXValueCreate(AXValueType.cgSize, &size)
                AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, sizeValue!)

                break
            }
        }
    }

    /// Capture screenshot and update the @Published cache
    @MainActor
    private func captureAndCacheScreenshot(windowId: CGWindowID, for taskspaceId: UUID) async {
        let startTime = Date()
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Starting screenshot capture for taskspace \(taskspaceId)"
        )

        // Use the screenshot manager to capture the screenshot directly
        if let screenshot = await screenshotManager.captureWindowScreenshot(windowId: windowId) {
            let captureTime = Date().timeIntervalSince(startTime)
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Screenshot captured in \(String(format: "%.3f", captureTime))s"
            )

            // Cache in memory for immediate UI updates
            taskspaceScreenshots[taskspaceId] = screenshot

            // Save to disk for persistence across app restarts
            await saveScreenshotToDisk(screenshot: screenshot, taskspaceId: taskspaceId)

            let totalTime = Date().timeIntervalSince(startTime)
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Screenshot cached for taskspace \(taskspaceId) (total: \(String(format: "%.3f", totalTime))s)"
            )
        } else {
            let failTime = Date().timeIntervalSince(startTime)
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to capture screenshot for taskspace \(taskspaceId) after \(String(format: "%.3f", failTime))s"
            )
        }
    }

    /// Save screenshot to disk for persistence across app restarts
    private func saveScreenshotToDisk(screenshot: NSImage, taskspaceId: UUID) async {
        guard let currentProject = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot save screenshot - no current project")
            return
        }

        // Find the taskspace to get its directory path
        guard let taskspace = currentProject.findTaskspace(uuid: taskspaceId.uuidString) else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot find taskspace for screenshot save: \(taskspaceId)"
            )
            return
        }

        let taskspaceDir = taskspace.directoryPath(in: currentProject.directoryPath)
        let screenshotPath = "\(taskspaceDir)/screenshot.png"

        // Convert NSImage to PNG data
        guard let tiffData = screenshot.tiffRepresentation,
            let bitmapImage = NSBitmapImageRep(data: tiffData),
            let pngData = bitmapImage.representation(using: .png, properties: [:])
        else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to convert screenshot to PNG data")
            return
        }

        do {
            try pngData.write(to: URL(fileURLWithPath: screenshotPath))
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Screenshot saved to disk: \(screenshotPath)")
        } catch {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to save screenshot to disk: \(error)")
        }
    }

    /// Load existing screenshots from disk on project open for visual persistence
    private func loadExistingScreenshots() {
        guard let currentProject = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Cannot load screenshots - no current project")
            return
        }

        var loadedCount = 0

        for taskspace in currentProject.taskspaces {
            let taskspaceDir = taskspace.directoryPath(in: currentProject.directoryPath)
            let screenshotPath = "\(taskspaceDir)/screenshot.png"

            if FileManager.default.fileExists(atPath: screenshotPath) {
                if let screenshot = NSImage(contentsOfFile: screenshotPath) {
                    taskspaceScreenshots[taskspace.id] = screenshot
                    loadedCount += 1
                    Logger.shared.log(
                        "ProjectManager[\(instanceId)]: Loaded screenshot from disk for taskspace: \(taskspace.name)"
                    )
                } else {
                    Logger.shared.log(
                        "ProjectManager[\(instanceId)]: Failed to load screenshot file: \(screenshotPath)"
                    )
                }
            }
        }

        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Loaded \(loadedCount) existing screenshots from disk")
    }

    // MARK: - Window Close Detection

    /// Start polling for closed windows to automatically transition taskspaces to Dormant state
    private func startWindowCloseDetection() {
        // Stop any existing timer
        stopWindowCloseDetection()

        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Starting window close detection (polling every 3 seconds)"
        )

        // Ensure timer is created on the main thread
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }

            self.windowCloseTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) {
                [weak self] _ in
                self?.checkForClosedWindows()
            }

            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Window close detection timer created successfully")
        }
    }

    /// Stop window close detection timer
    private func stopWindowCloseDetection() {
        DispatchQueue.main.async { [weak self] in
            self?.windowCloseTimer?.invalidate()
            self?.windowCloseTimer = nil
            Logger.shared.log(
                "ProjectManager[\(self?.instanceId ?? -1)]: Stopped window close detection"
            )
        }
    }

    /// Check if any registered windows have been closed and update taskspace states
    private func checkForClosedWindows() {
        let windowsToCheck = taskspaceWindows
        var closedWindows: [UUID] = []

        for (taskspaceId, windowId) in windowsToCheck {
            if !isWindowStillOpen(windowID: windowId) {
                closedWindows.append(taskspaceId)
            }
        }

        // Update state for closed windows
        for taskspaceId in closedWindows {
            if let taskspaceName = currentProject?.taskspaces.first(where: { $0.id == taskspaceId }
            )?.name {
                Logger.shared.log(
                    "ProjectManager[\(instanceId)]: Window closed for taskspace: \(taskspaceName)")
                taskspaceWindows.removeValue(forKey: taskspaceId)
                // Note: UI automatically updates via @Published taskspaceWindows and hasRegisteredWindow computed property
            }
        }
    }

    /// Check if a CGWindowID still exists in the system
    private func isWindowStillOpen(windowID: CGWindowID) -> Bool {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard
            let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID)
                as? [[String: Any]]
        else {
            return false
        }

        return windowList.contains { window in
            if let id = window[kCGWindowNumber as String] as? CGWindowID {
                return id == windowID
            }
            return false
        }
    }
}

// MARK: - IpcMessageDelegate

extension ProjectManager {

    func handleGetTaskspaceState(_ payload: GetTaskspaceStatePayload, messageId: String) async
        -> MessageHandlingResult<TaskspaceStateResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: No current project for get_taskspace_state")
            return .notForMe
        }

        // Look for taskspace with matching UUID in current project
        guard let taskspace = currentProject.findTaskspace(uuid: payload.taskspaceUuid) else {
            Logger.shared.log(
                "ProjectManager: Taskspace \(payload.taskspaceUuid) not found in project \(currentProject.name)"
            )
            return .notForMe
        }

        Logger.shared.log(
            "ProjectManager: Found taskspace \(taskspace.name) for UUID: \(payload.taskspaceUuid)")

        // Get agent command based on taskspace state and selected agent
        guard
            let agentCommand = agentManager.getAgentCommand(
                for: taskspace, selectedAgent: selectedAgent)
        else {
            Logger.shared.log(
                "ProjectManager: No valid agent command for taskspace \(taskspace.name)")
            return .notForMe
        }

        // Determine if agent should launch based on taskspace state
        // For now, always launch since we don't have a complete state
        let shouldLaunch = true

        let response = TaskspaceStateResponse(
            agentCommand: agentCommand,
            shouldLaunch: shouldLaunch
        )

        Logger.shared.log(
            "ProjectManager: Responding with shouldLaunch=\(shouldLaunch), command=\(agentCommand)")
        return .handled(response)
    }

    func handleSpawnTaskspace(_ payload: SpawnTaskspacePayload, messageId: String) async
        -> MessageHandlingResult<SpawnTaskspaceResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: No current project for spawn_taskspace")
            return .notForMe
        }

        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log(
                "ProjectManager: Project path mismatch: \(payload.projectPath) != \(currentProject.directoryPath)"
            )
            return .notForMe
        }

        Logger.shared.log(
            "ProjectManager: Creating taskspace \(payload.name) (parent UUID: \(payload.taskspaceUuid))"
        )

        do {
            // Generate comprehensive initial prompt using the user's task description from initialPrompt
            let comprehensivePrompt = generateInitialPrompt(taskDescription: payload.initialPrompt)

            // Use the existing createTaskspace logic
            try createTaskspace(
                name: payload.name,
                description: payload.taskDescription,
                initialPrompt: comprehensivePrompt
            )

            // Get the newly created taskspace (it will be the last one added)
            guard let newTaskspace = currentProject.taskspaces.last else {
                throw ProjectError.failedToSaveProject
            }

            // Return the new taskspace UUID in response
            let response = SpawnTaskspaceResponse(newTaskspaceUuid: newTaskspace.id.uuidString)
            return .handled(response)

        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: Failed to create taskspace: \(error)")
            return .notForMe
        }
    }

    func handleLogProgress(_ payload: LogProgressPayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager[\(instanceId)]: No current project for log_progress")
            return .notForMe
        }

        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log(
                "ProjectManager: Project path mismatch for log_progress: \(payload.projectPath) != \(currentProject.directoryPath)"
            )
            return .notForMe
        }

        // Find taskspace by UUID
        guard let taskspaceIndex = currentProject.findTaskspaceIndex(uuid: payload.taskspaceUuid)
        else {
            Logger.shared.log(
                "ProjectManager: Taskspace \(payload.taskspaceUuid) not found for log_progress")
            return .notForMe
        }

        Logger.shared.log(
            "ProjectManager: Adding log to taskspace \(payload.taskspaceUuid): \(payload.message)")

        do {
            // Create log entry
            let logCategory = LogCategory(rawValue: payload.category) ?? .info
            let logEntry = TaskspaceLog(message: payload.message, category: logCategory)

            // Update taskspace with new log
            var updatedProject = currentProject
            updatedProject.taskspaces[taskspaceIndex].logs.append(logEntry)

            // Transition from Hatchling state if needed
            if case .hatchling = updatedProject.taskspaces[taskspaceIndex].state {
                updatedProject.taskspaces[taskspaceIndex].state = .resume
            }

            // Save updated taskspace
            try updatedProject.taskspaces[taskspaceIndex].save(in: currentProject.directoryPath)

            // Capture screenshot when log is updated (if window is registered)
            if let windowID = taskspaceWindows[UUID(uuidString: payload.taskspaceUuid)!] {
                Task { @MainActor in
                    await captureAndCacheScreenshot(
                        windowId: windowID, for: UUID(uuidString: payload.taskspaceUuid)!)
                }
            }

            // Update UI
            DispatchQueue.main.async {
                self.currentProject = updatedProject
                Logger.shared.log("ProjectManager[\(self.instanceId)]: Updated taskspace logs")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: Failed to save log entry: \(error)")
            return .notForMe
        }
    }

    func handleSignalUser(_ payload: SignalUserPayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager[\(instanceId)]: No current project for signal_user")
            return .notForMe
        }

        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log(
                "ProjectManager: Project path mismatch for signal_user: \(payload.projectPath) != \(currentProject.directoryPath)"
            )
            return .notForMe
        }

        // Find taskspace by UUID
        guard let taskspaceIndex = currentProject.findTaskspaceIndex(uuid: payload.taskspaceUuid)
        else {
            Logger.shared.log(
                "ProjectManager: Taskspace \(payload.taskspaceUuid) not found for signal_user")
            return .notForMe
        }

        Logger.shared.log(
            "ProjectManager: Signaling user for taskspace \(payload.taskspaceUuid): \(payload.message)"
        )

        do {
            // Update taskspace with signal log entry
            var updatedProject = currentProject

            // Add a log entry to indicate user attention is needed
            let signalLog = TaskspaceLog(message: payload.message, category: .question)
            updatedProject.taskspaces[taskspaceIndex].logs.append(signalLog)

            // Transition from Hatchling state if needed
            if case .hatchling = updatedProject.taskspaces[taskspaceIndex].state {
                updatedProject.taskspaces[taskspaceIndex].state = .resume
            }

            // Save updated taskspace
            try updatedProject.taskspaces[taskspaceIndex].save(in: currentProject.directoryPath)

            // Update UI and dock badge
            DispatchQueue.main.async {
                self.currentProject = updatedProject

                // TODO: Update dock badge count
                // TODO: Bring app to foreground or show notification

                Logger.shared.log(
                    "ProjectManager[\(self.instanceId)]: Added signal log for user attention")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to update taskspace attention: \(error)")
            return .notForMe
        }
    }

    func handleUpdateTaskspace(_ payload: UpdateTaskspacePayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let project = currentProject else {
            return .notForMe
        }

        // Find the taskspace by UUID
        guard
            let taskspaceIndex = project.taskspaces.firstIndex(where: {
                $0.id.uuidString.lowercased() == payload.taskspaceUuid.lowercased()
            })
        else {
            Logger.shared.log(
                "ProjectManager: Taskspace not found for UUID: \(payload.taskspaceUuid)")
            return .notForMe
        }

        var updatedProject = project
        updatedProject.taskspaces[taskspaceIndex].name = payload.name
        updatedProject.taskspaces[taskspaceIndex].description = payload.description

        // Transition from Hatchling state if needed
        if case .hatchling = updatedProject.taskspaces[taskspaceIndex].state {
            updatedProject.taskspaces[taskspaceIndex].state = .resume
        }

        do {
            // Save updated taskspace to disk
            try updatedProject.taskspaces[taskspaceIndex].save(in: project.directoryPath)

            // Update UI
            DispatchQueue.main.async {
                self.currentProject = updatedProject
                Logger.shared.log(
                    "ProjectManager[\(self.instanceId)]: Updated taskspace: \(payload.name)")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to save taskspace update: \(error)")
            return .notForMe
        }
    }

    func handleDeleteTaskspace(_ payload: DeleteTaskspacePayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let project = currentProject else {
            return .notForMe
        }

        // Find the taskspace by UUID
        guard
            let taskspaceIndex = project.taskspaces.firstIndex(where: {
                $0.id.uuidString.lowercased() == payload.taskspaceUuid.lowercased()
            })
        else {
            Logger.shared.log(
                "ProjectManager: Taskspace not found for UUID: \(payload.taskspaceUuid)")
            return .notForMe
        }

        // Set the pendingDeletion flag to trigger UI confirmation dialog
        var updatedProject = project
        updatedProject.taskspaces[taskspaceIndex].pendingDeletion = true
        
        DispatchQueue.main.async {
            self.currentProject = updatedProject
            Logger.shared.log(
                "ProjectManager[\(self.instanceId)]: Triggered deletion dialog for taskspace: \(updatedProject.taskspaces[taskspaceIndex].name)")
        }
        
        return .handled(EmptyResponse())
    }

    /// Check if VSCode 'code' command is available and return its path
    private func getCodeCommandPath() -> String? {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/which")
        process.arguments = ["code"]

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()

        do {
            try process.run()
            process.waitUntilExit()

            if process.terminationStatus == 0 {
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8)?.trimmingCharacters(
                    in: .whitespacesAndNewlines)
                return output
            }
        } catch {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Failed to check for code command: \(error)")
        }

        return nil
    }

    /// Launch VSCode for a taskspace directory
    private func launchVSCode(for taskspace: Taskspace, in projectPath: String) {
        let taskspaceDir = taskspace.directoryPath(in: projectPath)
        let repoName = extractRepoName(from: currentProject?.gitURL ?? "")
        let workingDir = "\(taskspaceDir)/\(repoName)"

        // Check if the working directory exists
        guard FileManager.default.fileExists(atPath: workingDir) else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: Working directory not found for taskspace: \(taskspace.name)"
            )
            return
        }

        let vscodeProcess = Process()

        if let codePath = getCodeCommandPath() {
            // Use 'code' command - opens each directory in a new window by default
            vscodeProcess.executableURL = URL(fileURLWithPath: codePath)
            vscodeProcess.arguments = [workingDir]
            Logger.shared.log("ProjectManager[\(instanceId)]: Using code command at: \(codePath)")
        } else {
            // Fallback to 'open' command
            vscodeProcess.executableURL = URL(fileURLWithPath: "/usr/bin/open")
            vscodeProcess.arguments = ["-a", "Visual Studio Code", workingDir]
            Logger.shared.log("ProjectManager[\(instanceId)]: Code command not found, using open")
        }

        do {
            try vscodeProcess.run()
            Logger.shared.log(
                "ProjectManager: Launched VSCode for taskspace: \(taskspace.name) in \(repoName)"
            )
        } catch {
            Logger.shared.log(
                "ProjectManager: Failed to launch VSCode for \(taskspace.name): \(error)")
        }
    }
}
