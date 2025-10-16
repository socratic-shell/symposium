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
    
    // Pending deletion tracking - stores message IDs for taskspaces awaiting user confirmation
    private var pendingDeletionMessages: [UUID: String] = [:]

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
        
        // Set up Logger to send messages to daemon via IPC
        Logger.shared.setIpcManager(ipcManager)
        
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
        // stopWindowCloseDetection() // TODO: Implement this method
    }
    

    
    /// Execute a process and return results
    private func executeProcess(
        executable: String,
        arguments: [String],
        workingDirectory: String? = nil
    ) throws -> (exitCode: Int32, stdout: String, stderr: String) {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        
        if let workDir = workingDirectory {
            process.currentDirectoryURL = URL(fileURLWithPath: workDir)
        }
        
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        
        try process.run()
        process.waitUntilExit()
        
        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        
        let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
        let stderr = String(data: stderrData, encoding: .utf8) ?? ""
        
        return (process.terminationStatus, stdout, stderr)
    }

    private func executeProcessAsync(
        executable: String,
        arguments: [String],
        workingDirectory: String? = nil
    ) async throws -> (exitCode: Int32, stdout: String, stderr: String) {
        return try await withCheckedThrowingContinuation { continuation in
            Task.detached {
                do {
                    let result = try self.executeProcess(
                        executable: executable,
                        arguments: arguments,
                        workingDirectory: workingDirectory
                    )
                    continuation.resume(returning: result)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
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
        var project = try Project.load(from: directoryPath)

        // Set as current project first to display it
        setCurrentProject(project)
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

        // Validate taskspaces after display but before full interaction
        Task { @MainActor in
            await validateTaskspacesAsync(project)
        }

        // Start automatic window close detection
        self.startWindowCloseDetection()

        self.startMCPClient()
    }
    
    /// Validate taskspaces asynchronously after project is displayed
    @MainActor
    private func validateTaskspacesAsync(_ project: Project) async {
        let staleTaskspaces = findStaleTaskspaces(project.taskspaces, in: project.directoryPath, gitURL: project.gitURL)
        Logger.shared.log("ProjectManager[\(instanceId)]: Validated \(project.taskspaces.count) taskspaces, found \(staleTaskspaces.count) stale entries")
        
        if !staleTaskspaces.isEmpty {
            let shouldRemove = confirmStaleTaskspaceRemoval(staleTaskspaces)
            if shouldRemove {
                var updatedProject = project
                updatedProject.taskspaces = project.taskspaces.filter { taskspace in
                    !staleTaskspaces.contains { $0.id == taskspace.id }
                }
                Logger.shared.log("ProjectManager[\(instanceId)]: Removed \(staleTaskspaces.count) stale taskspace(s) from project")
                try? updatedProject.save()
                
                // Update the current project with cleaned taskspaces
                self.currentProject = updatedProject
            }
        }
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

    /// Find taskspaces with missing directories
    private func findStaleTaskspaces(_ taskspaces: [Taskspace], in projectPath: String, gitURL: String) -> [Taskspace] {
        let fileManager = FileManager.default
        let repoName = extractRepoName(from: gitURL)
        
        return taskspaces.filter { taskspace in
            let taskspaceDir = taskspace.directoryPath(in: projectPath)
            let taskspaceJsonPath = taskspace.taskspaceFilePath(in: projectPath)
            let workingDir = "\(taskspaceDir)/\(repoName)"
            
            let hasTaskspaceDir = fileManager.fileExists(atPath: taskspaceDir)
            let hasTaskspaceJson = fileManager.fileExists(atPath: taskspaceJsonPath)
            let hasWorkingDir = fileManager.fileExists(atPath: workingDir)
            
            let isValid = hasTaskspaceDir && hasTaskspaceJson && hasWorkingDir
            
            if !isValid {
                var missing: [String] = []
                if !hasTaskspaceDir { missing.append("directory") }
                if !hasTaskspaceJson { missing.append("taskspace.json") }
                if !hasWorkingDir { missing.append("worktree") }
                
                Logger.shared.log("ProjectManager[\(instanceId)]: Found stale taskspace: \(taskspace.name) (missing: \(missing.joined(separator: ", ")))")
            }
            
            return !isValid
        }
    }
    
    /// Show confirmation dialog for removing stale taskspaces
    @MainActor
    private func confirmStaleTaskspaceRemoval(_ staleTaskspaces: [Taskspace]) -> Bool {
        let taskspaceNames = staleTaskspaces.map { $0.name }.joined(separator: "\n• ")
        
        let alert = NSAlert()
        alert.messageText = "Remove Missing Taskspaces?"
        alert.informativeText = "The following taskspaces no longer have directories on disk:\n\n• \(taskspaceNames)\n\nWould you like to remove them from the project?"
        alert.addButton(withTitle: "Remove")
        alert.addButton(withTitle: "Keep")
        alert.alertStyle = .warning
        
        return alert.runModal() == .alertFirstButtonReturn
    }

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

        return taskspaces.sorted { $0.lastActivatedAt > $1.lastActivatedAt }
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
    /// Delete a taskspace including git worktree and optionally the branch
    ///
    /// Deletion workflow:
    /// 1. Compute paths: taskspaceDir (task-UUID) vs worktreeDir (task-UUID/reponame)
    /// 2. Get branch name before removal (needed for optional branch deletion)
    /// 3. Remove git worktree using worktreeDir (not taskspaceDir!) 
    /// 4. Fallback to directory removal if git worktree remove fails
    /// 5. Optionally delete branch if user chose to and branch exists
    /// 6. Update UI by removing taskspace from project model
    ///
    /// CRITICAL PATH RESOLUTION:
    /// - taskspaceDir = /path/task-UUID (taskspace directory)
    /// - worktreeDir = /path/task-UUID/reponame (actual git worktree)
    /// - Git commands must target worktreeDir and run from project.directoryPath (bare repo)
    func deleteTaskspace(_ taskspace: Taskspace, deleteBranch: Bool = false) async throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        await MainActor.run { isLoading = true }
        defer { Task { @MainActor in isLoading = false } }

        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        let repoName = extractRepoName(from: project.gitURL)
        let worktreeDir = "\(taskspaceDir)/\(repoName)"  // CRITICAL: Include repo name in path

        // Get current branch name before removing worktree (needed for optional branch deletion)
        let branchName = getTaskspaceBranch(for: taskspaceDir)

        // Remove git worktree - MUST use worktreeDir (includes repo name), not taskspaceDir
        // Command must run from bare repository directory (project.directoryPath)
        Logger.shared.log("Attempting to remove worktree: \(worktreeDir) from directory: \(project.directoryPath)")

        do {
            let result = try await executeProcessAsync(
                executable: "/usr/bin/git",
                arguments: ["worktree", "remove", worktreeDir, "--force"],
                workingDirectory: project.directoryPath
            )

            if result.exitCode != 0 {
                Logger.shared.log(
                    "Warning: Failed to remove git worktree, falling back to directory removal")
                // Fallback: remove directory if worktree removal failed
                try FileManager.default.removeItem(atPath: taskspaceDir)
            } else {
                Logger.shared.log("Successfully removed git worktree: \(worktreeDir)")
            }
        } catch {
            Logger.shared.log("Error executing worktree remove: \(error), falling back to directory removal")
            try FileManager.default.removeItem(atPath: taskspaceDir)
        }

        // Optionally delete the branch
        if deleteBranch && !branchName.isEmpty {
            do {
                let result = try await executeProcessAsync(
                    executable: "/usr/bin/git",
                    arguments: ["branch", "-D", branchName],
                    workingDirectory: project.directoryPath
                )

                if result.exitCode != 0 {
                    Logger.shared.log("Warning: Failed to delete branch \(branchName)")
                } else {
                    Logger.shared.log("ProjectManager[\(instanceId)]: Deleted branch \(branchName)")
                }
            }
        }

        // Remove from current project
        await MainActor.run {
            var updatedProject = project
            updatedProject.taskspaces.removeAll { $0.id == taskspace.id }
            self.currentProject = updatedProject
            Logger.shared.log(
                "ProjectManager[\(self.instanceId)]: Deleted taskspace \(taskspace.name)")
            
            // Send success response for pending deletion request
            self.sendDeletionConfirmedResponse(for: taskspace.id)
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

    /// Get comprehensive branch information for taskspace deletion safety checking
    /// 
    /// This method computes fresh branch info when called (not cached) because users may
    /// make commits between app startup and deletion attempts. Stale info could show
    /// incorrect warnings and lead to accidental data loss.
    ///
    /// Returns tuple with:
    /// - branchName: Current branch name from `git branch --show-current`
    /// - isMerged: Whether branch is merged into main via `git merge-base --is-ancestor`
    /// - unmergedCommits: Count of commits not in main via `git rev-list --count --not`
    /// - hasUncommittedChanges: Whether worktree has staged/unstaged changes via `git status --porcelain`
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

    /// Check if branch is merged into base branch
    /// 
    /// Note: getBaseBranch() returns just the branch name (e.g., "main"), 
    /// so we need to add remote prefix for remote comparison.
    private func isBranchMerged(branchName: String, baseBranch: String, in directory: String) throws -> Bool {
        guard let project = currentProject else { return false }
        
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        let remoteBranch = "\(project.remoteName)/\(baseBranch)"
        process.arguments = ["merge-base", "--is-ancestor", branchName, remoteBranch]
        process.currentDirectoryURL = URL(fileURLWithPath: directory)

        try process.run()
        process.waitUntilExit()

        return process.terminationStatus == 0
    }

    /// Count commits in branch that are not in base branch
    ///
    /// Uses `git rev-list --count <branch> --not <baseBranch>` to count unmerged commits.
    /// Note: getBaseBranch() returns just the branch name, so we add remote prefix.
    private func getUnmergedCommitCount(branchName: String, baseBranch: String, in directory: String) throws -> Int {
        guard let project = currentProject else { return 0 }
        
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        let remoteBranch = "\(project.remoteName)/\(baseBranch)"
        process.arguments = ["rev-list", "--count", "\(branchName)", "--not", remoteBranch]
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
    
    /// Check for uncommitted changes (both staged and unstaged)
    ///
    /// Uses `git status --porcelain` which detects:
    /// - Modified files (staged or unstaged)  
    /// - New files (staged or unstaged)
    /// - Deleted files (staged or unstaged)
    /// - Renamed files (staged or unstaged)
    /// Any output indicates uncommitted work that could be lost.
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
    func createTaskspace(name: String, description: String, initialPrompt: String, collaborator: String? = nil) throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        isLoading = true
        defer { isLoading = false }

        // Track completed stages for error reporting
        var completedStages: [String] = []
        

        
        Logger.shared.log("ProjectManager[\(instanceId)]: Starting taskspace creation for '\(name)'")

        // Create taskspace with provided values
        let taskspace = Taskspace(
            name: name,
            description: description,
            initialPrompt: initialPrompt,
            collaborator: collaborator
        )

        // STAGE 1: Create taskspace directory
        Logger.shared.log("ProjectManager[\(instanceId)]: Stage 1/5 - Creating taskspace directory")
        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        do {
            try FileManager.default.createDirectory(
                atPath: taskspaceDir,
                withIntermediateDirectories: true,
                attributes: nil
            )
            completedStages.append("Created taskspace directory")
            Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 1/5 completed - Directory created at \(taskspaceDir)")
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 1/5 failed - Directory creation failed")
            throw ProjectError.taskspaceDirectoryCreationFailed(
                taskspaceName: name,
                path: taskspaceDir,
                underlyingError: error
            )
        }

        // STAGE 2: Ensure bare repository exists (create if this is the first taskspace)
        Logger.shared.log("ProjectManager[\(instanceId)]: Stage 2/5 - Ensuring bare repository exists")
        if !bareRepositoryExists(in: project.repoPath) {
            Logger.shared.log("ProjectManager[\(instanceId)]: Creating bare repository for first taskspace")
            
            do {
                // Step 1: Clone bare into the main directory (not .git subdirectory)
                let cloneResult = try executeProcess(
                    executable: "/usr/bin/git",
                    arguments: ["clone", "--bare", project.gitURL, project.repoPath]
                )

                if cloneResult.exitCode != 0 {
                    Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 2/5 failed - Bare repository clone failed with exit code \(cloneResult.exitCode)")
                    throw ProjectError.bareRepositoryCreationFailed(
                        gitURL: project.gitURL,
                        targetPath: project.directoryPath,
                        exitCode: cloneResult.exitCode,
                        completedStages: completedStages
                    )
                }
                
                // Step 2: Set up remote tracking branches
                Logger.shared.log("ProjectManager[\(instanceId)]: Setting up remote tracking branches")
                let configResult = try executeProcess(
                    executable: "/usr/bin/git",
                    arguments: ["config", "remote.\(project.remoteName).fetch", "+refs/heads/*:refs/remotes/\(project.remoteName)/*"],
                    workingDirectory: project.directoryPath
                )
                
                if configResult.exitCode != 0 {
                    Logger.shared.log("ProjectManager[\(instanceId)]: ⚠️ Warning: Failed to configure remote tracking branches (exit code \(configResult.exitCode))")
                }
                
                // Step 3: Fetch remote to populate remote tracking branches
                Logger.shared.log("ProjectManager[\(instanceId)]: Fetching \(project.remoteName) to populate remote tracking branches")
                let fetchResult = try executeProcess(
                    executable: "/usr/bin/git",
                    arguments: ["fetch", project.remoteName],
                    workingDirectory: project.directoryPath
                )
                
                if fetchResult.exitCode != 0 {
                    Logger.shared.log("ProjectManager[\(instanceId)]: ⚠️ Warning: Failed to fetch \(project.remoteName) (exit code \(fetchResult.exitCode))")
                }
                
                // Step 4: Set up symbolic reference for remote/HEAD
                Logger.shared.log("ProjectManager[\(instanceId)]: Setting up symbolic reference for \(project.remoteName)/HEAD")
                let remoteResult = try executeProcess(
                    executable: "/usr/bin/git",
                    arguments: ["remote", "set-head", project.remoteName, "--auto"],
                    workingDirectory: project.directoryPath
                )
                
                if remoteResult.exitCode == 0 {
                    Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Symbolic reference set up successfully")
                } else {
                    Logger.shared.log("ProjectManager[\(instanceId)]: ⚠️ Warning: Failed to set up symbolic reference (exit code \(remoteResult.exitCode)), will use fallback detection")
                }
                
                completedStages.append("Created bare repository with proper setup")
                Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 2/5 completed - Bare repository created and configured")
            } catch let error as ProjectError {
                throw error
            } catch {
                Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 2/5 failed - Process execution failed: \(error)")
                throw ProjectError.bareRepositoryCreationFailed(
                    gitURL: project.gitURL,
                    targetPath: project.directoryPath,
                    exitCode: -1,
                    completedStages: completedStages
                )
            }
        } else {
            completedStages.append("Verified bare repository exists")
            Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 2/5 completed - Bare repository already exists")
        }

        // STAGE 3: Determine the base branch to start from
        Logger.shared.log("ProjectManager[\(instanceId)]: Stage 3/5 - Detecting base branch")
        let baseBranch: String
        do {
            baseBranch = try getBaseBranch(for: project)
            completedStages.append("Detected base branch: \(baseBranch)")
            Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 3/5 completed - Base branch: \(baseBranch)")
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 3/5 failed - Base branch detection failed")
            throw ProjectError.baseBranchDetectionFailed(
                projectPath: project.directoryPath,
                completedStages: completedStages
            )
        }

        // STAGE 4: Create worktree for this taskspace with unique branch
        Logger.shared.log("ProjectManager[\(instanceId)]: Stage 4/5 - Creating git worktree")
        // CAREFUL: When adding new steps to taskspace creation, you likely need to modify `findStaleTaskspaces` as well to check for this.
        let branchName = "taskspace-\(taskspace.id.uuidString)"
        let repoName = extractRepoName(from: project.gitURL)
        let worktreeDir = "\(taskspaceDir)/\(repoName)"

        Logger.shared.log("ProjectManager[\(instanceId)]: Creating worktree with branch \(branchName) from \(baseBranch)")

        do {
            let result = try executeProcess(
                executable: "/usr/bin/git",
                arguments: ["worktree", "add", worktreeDir, "-b", branchName, baseBranch],
                workingDirectory: project.directoryPath
            )

            if result.exitCode != 0 {
                Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 4/5 failed - Worktree creation failed with exit code \(result.exitCode)")
                throw ProjectError.worktreeCreationFailed(
                    branchName: branchName,
                    worktreePath: worktreeDir,
                    baseBranch: baseBranch,
                    exitCode: result.exitCode,
                    completedStages: completedStages
                )
            }
            
            completedStages.append("Created git worktree and branch")
            Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 4/5 completed - Worktree created at \(worktreeDir)")
        } catch let error as ProjectError {
            throw error
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 4/5 failed - Process execution failed: \(error)")
            throw ProjectError.worktreeCreationFailed(
                branchName: branchName,
                worktreePath: worktreeDir,
                baseBranch: baseBranch,
                exitCode: -1,
                completedStages: completedStages
            )
        }

        // STAGE 5: Save taskspace metadata
        Logger.shared.log("ProjectManager[\(instanceId)]: Stage 5/5 - Saving taskspace metadata")
        do {
            try taskspace.save(in: project.directoryPath)
            completedStages.append("Saved taskspace metadata")
            Logger.shared.log("ProjectManager[\(instanceId)]: ✅ Stage 5/5 completed - Metadata saved")
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: ❌ Stage 5/5 failed - Metadata save failed")
            throw ProjectError.taskspaceMetadataSaveFailed(
                taskspaceName: name,
                projectPath: project.directoryPath,
                underlyingError: error,
                completedStages: completedStages
            )
        }

        // Add to current project
        DispatchQueue.main.async {
            var updatedProject = project
            updatedProject.taskspaces.append(taskspace)
            self.currentProject = updatedProject
        }

        // Auto-activate new taskspace by launching VSCode
        launchVSCode(for: taskspace, in: project.directoryPath)
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: 🎉 Successfully created and activated taskspace '\(taskspace.name)' - All 5 stages completed"
        )
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
            Logger.shared.log("ProjectManager[\(instanceId)]: Using configured default branch: \(defaultBranch)")
            return defaultBranch
        }

        Logger.shared.log("ProjectManager[\(instanceId)]: No default branch configured, auto-detecting from git")

        // Auto-detect remote's default branch
        do {
            let result = try executeProcess(
                executable: "/usr/bin/git",
                arguments: ["symbolic-ref", "refs/remotes/\(project.remoteName)/HEAD"],
                workingDirectory: project.directoryPath
            )

            if result.exitCode == 0 {
                let output = result.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
                // Output is like "refs/remotes/origin/main", extract just "main"
                let remotePrefix = "refs/remotes/\(project.remoteName)/"
                if output.hasPrefix(remotePrefix) {
                    let branchName = String(output.dropFirst(remotePrefix.count))
                    Logger.shared.log("ProjectManager[\(instanceId)]: Auto-detected base branch: \(branchName)")
                    return branchName
                }
            } else {
                Logger.shared.log("ProjectManager[\(instanceId)]: Git symbolic-ref failed with exit code \(result.exitCode)")
            }
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: Failed to execute git symbolic-ref: \(error.localizedDescription)")
        }

        // Try alternative method: check available remote branches
        Logger.shared.log("ProjectManager[\(instanceId)]: Trying alternative method to detect base branch")
        do {
            let result = try executeProcess(
                executable: "/usr/bin/git",
                arguments: ["branch", "-r"],
                workingDirectory: project.directoryPath
            )

            if result.exitCode == 0 {
                let branches = result.stdout.components(separatedBy: .newlines)
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty && !$0.contains("->") }
                
                // Look for common default branch names and return just the branch name
                let remotePrefix = "\(project.remoteName)/"
                for commonBranch in ["\(project.remoteName)/main", "\(project.remoteName)/master", "\(project.remoteName)/develop"] {
                    if branches.contains(commonBranch) {
                        let branchName = String(commonBranch.dropFirst(remotePrefix.count))
                        Logger.shared.log("ProjectManager[\(instanceId)]: Found common branch: \(commonBranch), using: \(branchName)")
                        return branchName
                    }
                }
                
                // Use the first available remote branch
                if let firstRemoteBranch = branches.first(where: { $0.hasPrefix(remotePrefix) }) {
                    let branchName = String(firstRemoteBranch.dropFirst(remotePrefix.count))
                    Logger.shared.log("ProjectManager[\(instanceId)]: Using first available \(project.remoteName) branch: \(firstRemoteBranch), extracted: \(branchName)")
                    return branchName
                }
                
                Logger.shared.log("ProjectManager[\(instanceId)]: Available remote branches: \(branches.joined(separator: ", "))")
            }
        } catch {
            Logger.shared.log("ProjectManager[\(instanceId)]: Failed to list remote branches: \(error.localizedDescription)")
        }

        // Final fallback to main
        Logger.shared.log(
            "ProjectManager[\(instanceId)]: Could not detect any suitable base branch, falling back to main"
        )
        return "main"
    }

    /// Check if a bare git repository exists at the project path
    private func bareRepositoryExists(in projectPath: String) -> Bool {
        let fileManager = FileManager.default

        // Check if config file exists (bare repos have config in root, not .git subdirectory)
        let configPath = "\(projectPath)/config"
        guard fileManager.fileExists(atPath: configPath) else {
            return false
        }

        // Check if it's a bare repository by looking for the 'bare' config
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
    
    // Enhanced taskspace creation errors with context
    case taskspaceDirectoryCreationFailed(taskspaceName: String, path: String, underlyingError: Error)
    case bareRepositoryCreationFailed(gitURL: String, targetPath: String, exitCode: Int32, completedStages: [String])
    case baseBranchDetectionFailed(projectPath: String, completedStages: [String])
    case worktreeCreationFailed(branchName: String, worktreePath: String, baseBranch: String, exitCode: Int32, completedStages: [String])
    case taskspaceMetadataSaveFailed(taskspaceName: String, projectPath: String, underlyingError: Error, completedStages: [String])

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
            
        case .taskspaceDirectoryCreationFailed(let taskspaceName, let path, let underlyingError):
            return """
            Failed to create taskspace directory during taskspace creation.
            
            Taskspace: '\(taskspaceName)'
            Target path: \(path)
            Stage: Creating taskspace directory (step 1 of 5)
            Completed stages: None
            
            Underlying error: \(underlyingError.localizedDescription)
            
            This usually indicates a filesystem permission issue or insufficient disk space.
            """;
            
        case .bareRepositoryCreationFailed(let gitURL, let targetPath, let exitCode, let completedStages):
            return """
            Failed to create bare git repository during taskspace creation.
            
            Git URL: \(gitURL)
            Target path: \(targetPath)
            Stage: Creating bare repository (step 2 of 5)
            Completed stages: \(completedStages.joined(separator: ", "))
            Git exit code: \(exitCode)
            
            This usually indicates:
            - Network connectivity issues
            - Invalid git URL or authentication problems
            - Insufficient disk space
            - Git is not installed or not in PATH
            """
            
        case .baseBranchDetectionFailed(let projectPath, let completedStages):
            return """
            Failed to detect base branch during taskspace creation.
            
            Project path: \(projectPath)
            Stage: Detecting base branch (step 3 of 5)
            Completed stages: \(completedStages.joined(separator: ", "))
            
            This usually indicates:
            - The bare repository was not created properly
            - No remote branches are available
            - Git configuration issues
            """
            
        case .worktreeCreationFailed(let branchName, let worktreePath, let baseBranch, let exitCode, let completedStages):
            return """
            Failed to create git worktree during taskspace creation.
            
            Branch name: \(branchName)
            Worktree path: \(worktreePath)
            Base branch: \(baseBranch)
            Stage: Creating git worktree (step 4 of 5)
            Completed stages: \(completedStages.joined(separator: ", "))
            Git exit code: \(exitCode)
            
            This usually indicates:
            - The base branch '\(baseBranch)' does not exist
            - Branch name '\(branchName)' already exists
            - Filesystem permission issues
            - Corrupted git repository
            """
            
        case .taskspaceMetadataSaveFailed(let taskspaceName, let projectPath, let underlyingError, let completedStages):
            return """
            Failed to save taskspace metadata during taskspace creation.
            
            Taskspace: '\(taskspaceName)'
            Project path: \(projectPath)
            Stage: Saving taskspace metadata (step 5 of 5)
            Completed stages: \(completedStages.joined(separator: ", "))
            
            Underlying error: \(underlyingError.localizedDescription)
            
            The git worktree was created successfully, but metadata could not be saved.
            This usually indicates filesystem permission issues.
            """
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

    func handleTaskspaceState(_ payload: TaskspaceStateRequest, messageId: String) async
        -> MessageHandlingResult<TaskspaceStateResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log(
                "ProjectManager[\(instanceId)]: No current project for taskspace_state")
            return .notForMe
        }

        // Look for taskspace with matching UUID in current project
        guard let taskspaceIndex = currentProject.taskspaces.firstIndex(where: { 
            $0.id.uuidString.lowercased() == payload.taskspaceUuid.lowercased() 
        }) else {
            Logger.shared.log(
                "ProjectManager: Taskspace \(payload.taskspaceUuid) not found in project \(currentProject.name)"
            )
            return .notForMe
        }

        var updatedProject = currentProject
        var taskspace = updatedProject.taskspaces[taskspaceIndex]

        Logger.shared.log(
            "ProjectManager: Found taskspace \(taskspace.name) for UUID: \(payload.taskspaceUuid)")

        // Handle update operation if name or description provided
        var hasUpdates = false
        if let newName = payload.name {
            taskspace.name = newName
            hasUpdates = true
            Logger.shared.log("ProjectManager: Updated taskspace name to: \(newName)")
        }
        
        if let newDescription = payload.description {
            taskspace.description = newDescription
            hasUpdates = true
            Logger.shared.log("ProjectManager: Updated taskspace description to: \(newDescription)")
        }
        
        if let newCollaborator = payload.collaborator {
            taskspace.collaborator = newCollaborator
            hasUpdates = true
            Logger.shared.log("ProjectManager: Updated taskspace collaborator to: \(newCollaborator)")
        }

        // Determine initial_prompt based on operation type
        let initialPrompt: String?
        if hasUpdates {
            // This is an update operation - clear initial_prompt by transitioning state
            if case .hatchling = taskspace.state {
                taskspace.state = .resume
                Logger.shared.log("ProjectManager: Transitioned taskspace from hatchling to resume state")
            }
            initialPrompt = nil
            Logger.shared.log("ProjectManager: Clearing initial_prompt after update operation")
            
            // Save changes to disk and update UI
            updatedProject.taskspaces[taskspaceIndex] = taskspace
            do {
                try taskspace.save(in: currentProject.directoryPath)
                DispatchQueue.main.async {
                    self.currentProject = updatedProject
                }
            } catch {
                Logger.shared.log("ProjectManager: Failed to save taskspace changes: \(error)")
            }
        } else {
            // This is a read operation - return current initial_prompt
            initialPrompt = taskspace.initialPrompt
            Logger.shared.log("ProjectManager: Returning initial_prompt for read operation")
        }

        // Get agent command based on taskspace state and selected agent
        guard
            let agentCommand = agentManager.getAgentCommand(
                for: taskspace, selectedAgent: selectedAgent)
        else {
            Logger.shared.log(
                "ProjectManager: No valid agent command for taskspace \(taskspace.name)")
            return .notForMe
        }

        let response = TaskspaceStateResponse(
            name: taskspace.name,
            description: taskspace.description,
            initialPrompt: initialPrompt,
            agentCommand: agentCommand,
            collaborator: taskspace.collaborator
        )

        Logger.shared.log(
            "ProjectManager: Responding with name=\(taskspace.name), description=\(taskspace.description), initialPrompt=\(initialPrompt != nil ? "present" : "nil"), agentCommand=\(agentCommand)")
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
                initialPrompt: comprehensivePrompt,
                collaborator: payload.collaborator
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

        let taskspace = project.taskspaces[taskspaceIndex]
        
        // Store the message ID for later response when dialog completes
        pendingDeletionMessages[taskspace.id] = messageId

        // Set the pendingDeletion flag to trigger UI confirmation dialog
        var updatedProject = project
        updatedProject.taskspaces[taskspaceIndex].pendingDeletion = true
        
        DispatchQueue.main.async {
            self.currentProject = updatedProject
            Logger.shared.log(
                "ProjectManager[\(self.instanceId)]: Triggered deletion dialog for taskspace: \(updatedProject.taskspaces[taskspaceIndex].name), awaiting user confirmation")
        }
        
        // Don't return a response yet - wait for user confirmation/cancellation
        // The response will be sent when the dialog completes
        return .pending
    }
    
    /// Send success response for a pending taskspace deletion
    private func sendDeletionConfirmedResponse(for taskspaceId: UUID) {
        guard let messageId = pendingDeletionMessages.removeValue(forKey: taskspaceId) else {
            Logger.shared.log("ProjectManager[\(instanceId)]: No pending message found for taskspace deletion confirmation")
            return
        }
        
        ipcManager.sendResponse(
            to: messageId, 
            success: true, 
            data: EmptyResponse(), 
            error: nil
        )
        
        Logger.shared.log("ProjectManager[\(instanceId)]: Sent deletion confirmed response for taskspace")
    }
    
    /// Send cancellation response for a pending taskspace deletion
    func sendDeletionCancelledResponse(for taskspaceId: UUID) {
        guard let messageId = pendingDeletionMessages.removeValue(forKey: taskspaceId) else {
            Logger.shared.log("ProjectManager[\(instanceId)]: No pending message found for taskspace deletion cancellation")
            return
        }
        
        ipcManager.sendResponse(
            to: messageId, 
            success: false, 
            data: nil as String?, 
            error: "Taskspace deletion was cancelled by user"
        )
        
        Logger.shared.log("ProjectManager[\(instanceId)]: Sent deletion cancelled response for taskspace")
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
