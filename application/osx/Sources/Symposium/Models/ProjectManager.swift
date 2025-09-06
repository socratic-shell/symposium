import AppKit
import CoreGraphics
import Foundation

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

    // Window associations for current project
    @Published private var taskspaceWindows: [UUID: CGWindowID] = [:]
    
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
        self.agentManager = agentManager
        self.settingsManager = settingsManager
        self.selectedAgent = selectedAgent
        self.permissionManager = permissionManager

        // ScreenshotManager initialization is deferred via lazy var
    }
    
    deinit {
        stopWindowCloseDetection()
    }

    /// Create a new Symposium project
    func createProject(name: String, gitURL: String, at directoryPath: String) throws {
        isLoading = true
        defer { isLoading = false }

        // Create project directory with .symposium extension
        let projectDirPath = "\(directoryPath)/\(name).symposium"

        // Check if directory already exists
        if FileManager.default.fileExists(atPath: projectDirPath) {
            throw ProjectError.directoryAlreadyExists
        }

        // Create directory
        try FileManager.default.createDirectory(
            atPath: projectDirPath,
            withIntermediateDirectories: true,
            attributes: nil
        )

        // Create project instance
        let project = Project(name: name, gitURL: gitURL, directoryPath: projectDirPath)

        // Save project.json
        try project.save()

        // Set as current project
        setCurrentProject(project)
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
        Logger.shared.log("ProjectManager: Registered as IPC delegate for project: \(project.name)")

        // Phase 30: Do NOT auto-launch VSCode - taskspaces start dormant until user activates them
        Logger.shared.log("ProjectManager: Project opened with \(project.taskspaces.count) dormant taskspaces")

        // Load existing screenshots from disk for visual persistence
        self.loadExistingScreenshots()
        
        // Start automatic window close detection
        self.startWindowCloseDetection()

        self.startMCPClient()
    }

    /// Launch VSCode for a specific taskspace (used for user-activated awakening)
    func launchVSCode(for taskspace: Taskspace) {
        guard let project = currentProject else {
            Logger.shared.log("ProjectManager: Cannot launch VSCode - no current project")
            return
        }
        
        launchVSCode(for: taskspace, in: project.directoryPath)
        Logger.shared.log("ProjectManager: User-activated VSCode for taskspace: \(taskspace.name)")
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
        // Unregister as IPC delegate
        ipcManager.removeDelegate(self)
        Logger.shared.log("ProjectManager: Unregistered as IPC delegate")

        stopMCPClient()
        DispatchQueue.main.async {
            self.currentProject = nil
            self.errorMessage = nil
        }
    }

    private func startMCPClient() {
        Logger.shared.log("ProjectManager: Starting daemon client")
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
                Logger.shared.log("ProjectManager: Starting daemon with path: \(mcpPath)")
                ipcManager.startClient(mcpServerPath: mcpPath)
            } else {
                Logger.shared.log(
                    "ProjectManager: Agent not ready - installed: \(selectedAgentInfo.isInstalled), mcpConfigured: \(selectedAgentInfo.isMCPConfigured), mcpPath: \(selectedAgentInfo.mcpServerPath ?? "nil")"
                )
            }
        } else {
            Logger.shared.log("ProjectManager: No agent found with id: \(selectedAgent)")
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
    func deleteTaskspace(_ taskspace: Taskspace) throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        isLoading = true
        defer { isLoading = false }

        // Delete the taskspace directory recursively
        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        try FileManager.default.removeItem(atPath: taskspaceDir)

        // Remove from current project
        DispatchQueue.main.async {
            var updatedProject = project
            updatedProject.taskspaces.removeAll { $0.id == taskspace.id }
            self.currentProject = updatedProject
            Logger.shared.log("ProjectManager: Deleted taskspace \(taskspace.name)")
        }
    }

    /// Create a new taskspace with default values
    func createTaskspace() throws {
        guard let project = currentProject else {
            throw ProjectError.noCurrentProject
        }

        isLoading = true
        defer { isLoading = false }

        // Create taskspace with default values
        let taskspace = Taskspace(
            name: "Unnamed taskspace",
            description: "TBD",
            initialPrompt:
                "This is a newly created taskspace. Figure out what the user wants to do and update the name/description appropriately using the `update_taskspace` tool."
        )

        // Create taskspace directory
        let taskspaceDir = taskspace.directoryPath(in: project.directoryPath)
        try FileManager.default.createDirectory(
            atPath: taskspaceDir,
            withIntermediateDirectories: true,
            attributes: nil
        )

        // Clone repository into taskspace directory
        let repoName = extractRepoName(from: project.gitURL)
        let cloneDir = "\(taskspaceDir)/\(repoName)"

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["clone", project.gitURL, cloneDir]

        try process.run()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            throw ProjectError.gitCloneFailed
        }

        // Save taskspace metadata
        try taskspace.save(in: project.directoryPath)

        // Phase 30: Do NOT auto-launch VSCode - new taskspaces start dormant until user clicks
        Logger.shared.log("ProjectManager: Created new taskspace '\(taskspace.name)' (dormant until activated)")

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
            Logger.shared.log("ProjectManager: Invalid UUID format: \(taskspaceUuid)")
            return false
        }

        // Verify taskspace exists in current project
        guard let project = currentProject,
            project.findTaskspace(uuid: taskspaceUuid) != nil
        else {
            Logger.shared.log("ProjectManager: Taskspace not found for UUID: \(taskspaceUuid)")
            return false
        }

        taskspaceWindows[uuid] = windowID
        Logger.shared.log("ProjectManager: Associated window \(windowID) with taskspace \(uuid)")

        // Capture screenshot when window is first registered
        Logger.shared.log(
            "ProjectManager: Attempting screenshot capture for window \(windowID), taskspace \(uuid)"
        )
        Task { @MainActor in
            Logger.shared.log("ProjectManager: Starting screenshot capture task")
            await captureAndCacheScreenshot(windowId: windowID, for: uuid)
        }

        return true
    }

    /// Get window ID for a taskspace
    func getWindow(for taskspaceUuid: UUID) -> CGWindowID? {
        return taskspaceWindows[taskspaceUuid]
    }

    /// Capture screenshot and update the @Published cache
    @MainActor
    private func captureAndCacheScreenshot(windowId: CGWindowID, for taskspaceId: UUID) async {
        let startTime = Date()
        Logger.shared.log("ProjectManager: Starting screenshot capture for taskspace \(taskspaceId)")
        
        // Use the screenshot manager to capture the screenshot directly
        if let screenshot = await screenshotManager.captureWindowScreenshot(windowId: windowId) {
            let captureTime = Date().timeIntervalSince(startTime)
            Logger.shared.log("ProjectManager: Screenshot captured in \(String(format: "%.3f", captureTime))s")
            
            // Cache in memory for immediate UI updates
            taskspaceScreenshots[taskspaceId] = screenshot
            
            // Save to disk for persistence across app restarts
            await saveScreenshotToDisk(screenshot: screenshot, taskspaceId: taskspaceId)
            
            let totalTime = Date().timeIntervalSince(startTime)
            Logger.shared.log("ProjectManager: Screenshot cached for taskspace \(taskspaceId) (total: \(String(format: "%.3f", totalTime))s)")
        } else {
            let failTime = Date().timeIntervalSince(startTime)
            Logger.shared.log("ProjectManager: Failed to capture screenshot for taskspace \(taskspaceId) after \(String(format: "%.3f", failTime))s")
        }
    }
    
    /// Save screenshot to disk for persistence across app restarts
    private func saveScreenshotToDisk(screenshot: NSImage, taskspaceId: UUID) async {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: Cannot save screenshot - no current project")
            return
        }
        
        // Find the taskspace to get its directory path
        guard let taskspace = currentProject.findTaskspace(uuid: taskspaceId.uuidString) else {
            Logger.shared.log("ProjectManager: Cannot find taskspace for screenshot save: \(taskspaceId)")
            return
        }
        
        let taskspaceDir = taskspace.directoryPath(in: currentProject.directoryPath)
        let screenshotPath = "\(taskspaceDir)/screenshot.png"
        
        // Convert NSImage to PNG data
        guard let tiffData = screenshot.tiffRepresentation,
              let bitmapImage = NSBitmapImageRep(data: tiffData),
              let pngData = bitmapImage.representation(using: .png, properties: [:]) else {
            Logger.shared.log("ProjectManager: Failed to convert screenshot to PNG data")
            return
        }
        
        do {
            try pngData.write(to: URL(fileURLWithPath: screenshotPath))
            Logger.shared.log("ProjectManager: Screenshot saved to disk: \(screenshotPath)")
        } catch {
            Logger.shared.log("ProjectManager: Failed to save screenshot to disk: \(error)")
        }
    }
    
    /// Load existing screenshots from disk on project open for visual persistence
    private func loadExistingScreenshots() {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: Cannot load screenshots - no current project")
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
                    Logger.shared.log("ProjectManager: Loaded screenshot from disk for taskspace: \(taskspace.name)")
                } else {
                    Logger.shared.log("ProjectManager: Failed to load screenshot file: \(screenshotPath)")
                }
            }
        }
        
        Logger.shared.log("ProjectManager: Loaded \(loadedCount) existing screenshots from disk")
    }
    
    // MARK: - Window Close Detection
    
    /// Start polling for closed windows to automatically transition taskspaces to Dormant state
    private func startWindowCloseDetection() {
        // Stop any existing timer
        stopWindowCloseDetection()
        
        Logger.shared.log("ProjectManager: Starting window close detection (polling every 3 seconds)")
        
        windowCloseTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { [weak self] _ in
            self?.checkForClosedWindows()
        }
    }
    
    /// Stop window close detection timer
    private func stopWindowCloseDetection() {
        windowCloseTimer?.invalidate()
        windowCloseTimer = nil
        Logger.shared.log("ProjectManager: Stopped window close detection")
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
            if let taskspaceName = currentProject?.taskspaces.first(where: { $0.id == taskspaceId })?.name {
                Logger.shared.log("ProjectManager: Window closed for taskspace: \(taskspaceName)")
                taskspaceWindows.removeValue(forKey: taskspaceId)
                // Note: UI automatically updates via @Published taskspaceWindows and hasRegisteredWindow computed property
            }
        }
    }
    
    /// Check if a CGWindowID still exists in the system
    private func isWindowStillOpen(windowID: CGWindowID) -> Bool {
        let options = CGWindowListOption(arrayLiteral: .excludeDesktopElements)
        guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
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
            Logger.shared.log("ProjectManager: No current project for get_taskspace_state")
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
            Logger.shared.log("ProjectManager: No current project for spawn_taskspace")
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
            // Create new taskspace with fresh UUID
            let taskspace = Taskspace(
                name: payload.name,
                description: payload.taskDescription,
                initialPrompt: payload.initialPrompt
            )

            Logger.shared.log(
                "ProjectManager: Created new taskspace with UUID: \(taskspace.id.uuidString)")

            // Create taskspace directory and clone repo
            // TODO: This should use the existing createTaskspace logic
            let taskspaceDir = taskspace.directoryPath(in: currentProject.directoryPath)
            try FileManager.default.createDirectory(
                atPath: taskspaceDir,
                withIntermediateDirectories: true,
                attributes: nil
            )

            // Save taskspace metadata
            try taskspace.save(in: currentProject.directoryPath)

            // Update current project with new taskspace
            DispatchQueue.main.async {
                var updatedProject = currentProject
                updatedProject.taskspaces.append(taskspace)
                self.currentProject = updatedProject
                Logger.shared.log("ProjectManager: Added taskspace \(taskspace.name) to project")
            }

            // Return the new taskspace UUID in response
            let response = SpawnTaskspaceResponse(newTaskspaceUuid: taskspace.id.uuidString)
            return .handled(response)

        } catch {
            Logger.shared.log("ProjectManager: Failed to create taskspace: \(error)")
            return .notForMe
        }
    }

    func handleLogProgress(_ payload: LogProgressPayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for log_progress")
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
                Logger.shared.log("ProjectManager: Updated taskspace logs")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log("ProjectManager: Failed to save log entry: \(error)")
            return .notForMe
        }
    }

    func handleSignalUser(_ payload: SignalUserPayload, messageId: String) async
        -> MessageHandlingResult<EmptyResponse>
    {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for signal_user")
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

                Logger.shared.log("ProjectManager: Added signal log for user attention")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log("ProjectManager: Failed to update taskspace attention: \(error)")
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
                Logger.shared.log("ProjectManager: Updated taskspace: \(payload.name)")
            }

            return .handled(EmptyResponse())

        } catch {
            Logger.shared.log("ProjectManager: Failed to save taskspace update: \(error)")
            return .notForMe
        }
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
            Logger.shared.log("ProjectManager: Failed to check for code command: \(error)")
        }

        return nil
    }

    /// Launch VSCode for a taskspace directory
    private func launchVSCode(for taskspace: Taskspace, in projectPath: String) {
        let taskspaceDir = taskspace.directoryPath(in: projectPath)

        // Find the cloned repository directory within the taskspace
        do {
            let contents = try FileManager.default.contentsOfDirectory(atPath: taskspaceDir)
            // Look for a directory that's not taskspace.json
            if let repoDir = contents.first(where: { $0 != "taskspace.json" }) {
                let cloneDir = "\(taskspaceDir)/\(repoDir)"

                let vscodeProcess = Process()

                if let codePath = getCodeCommandPath() {
                    // Use 'code' command - opens each directory in a new window by default
                    vscodeProcess.executableURL = URL(fileURLWithPath: codePath)
                    vscodeProcess.arguments = [cloneDir]
                    Logger.shared.log("ProjectManager: Using code command at: \(codePath)")
                } else {
                    // Fallback to 'open' command
                    vscodeProcess.executableURL = URL(fileURLWithPath: "/usr/bin/open")
                    vscodeProcess.arguments = ["-a", "Visual Studio Code", cloneDir]
                    Logger.shared.log("ProjectManager: Code command not found, using open")
                }

                try vscodeProcess.run()
                Logger.shared.log(
                    "ProjectManager: Launched VSCode for taskspace: \(taskspace.name) in \(repoDir)"
                )
            } else {
                Logger.shared.log(
                    "ProjectManager: No repository directory found for taskspace: \(taskspace.name)"
                )
            }
        } catch {
            Logger.shared.log(
                "ProjectManager: Failed to launch VSCode for \(taskspace.name): \(error)")
        }
    }
}
