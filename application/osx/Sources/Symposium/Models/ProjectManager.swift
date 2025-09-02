import Foundation

/// Manages project creation, loading, and operations
class ProjectManager: ObservableObject, IpcMessageDelegate {
    @Published var currentProject: Project?
    @Published var isLoading = false
    @Published var errorMessage: String?
    
    private let ipcManager = IpcManager()
    private let agentManager: AgentManager
    private let settingsManager: SettingsManager
    private let selectedAgent: String
    
    var mcpStatus: IpcManager { ipcManager }
    
    init(agentManager: AgentManager, settingsManager: SettingsManager, selectedAgent: String) {
        self.agentManager = agentManager
        self.settingsManager = settingsManager
        self.selectedAgent = selectedAgent
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
        DispatchQueue.main.async {
            self.currentProject = project
            self.errorMessage = nil
            
            // Save project path for next app launch
            self.settingsManager.lastProjectPath = project.directoryPath
            
            // Register as IPC delegate for this project
            self.ipcManager.addDelegate(self)
            Logger.shared.log("ProjectManager: Registered as IPC delegate for project: \(project.name)")
            
            // Launch VSCode for active taskspaces
            self.launchVSCodeForActiveTaskspaces(in: project)
            
            self.startMCPClient()
        }
    }
    
    /// Launch VSCode for all active taskspaces (both hatchling and resume states)
    private func launchVSCodeForActiveTaskspaces(in project: Project) {
        let activeTaskspaces = project.taskspaces.filter { taskspace in
            switch taskspace.state {
            case .hatchling, .resume:
                return true
            }
        }
        
        for taskspace in activeTaskspaces {
            launchVSCode(for: taskspace, in: project.directoryPath)
        }
        
        if !activeTaskspaces.isEmpty {
            Logger.shared.log("ProjectManager: Launched VSCode for \(activeTaskspaces.count) active taskspaces")
        }
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
        if let selectedAgentInfo = agentManager.availableAgents.first(where: { $0.id == selectedAgent }) {
            Logger.shared.log("ProjectManager: Found agent \(selectedAgent): installed=\(selectedAgentInfo.isInstalled), mcpConfigured=\(selectedAgentInfo.isMCPConfigured)")
            
            if selectedAgentInfo.isInstalled && selectedAgentInfo.isMCPConfigured,
               let mcpPath = selectedAgentInfo.mcpServerPath {
                Logger.shared.log("ProjectManager: Starting daemon with path: \(mcpPath)")
                ipcManager.startClient(mcpServerPath: mcpPath)
            } else {
                Logger.shared.log("ProjectManager: Agent not ready - installed: \(selectedAgentInfo.isInstalled), mcpConfigured: \(selectedAgentInfo.isMCPConfigured), mcpPath: \(selectedAgentInfo.mcpServerPath ?? "nil")")
            }
        } else {
            Logger.shared.log("ProjectManager: No agent found with id: \(selectedAgent)")
            Logger.shared.log("ProjectManager: Available agents: \(agentManager.availableAgents.map { $0.id })")
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
            initialPrompt: "This is a newly created taskspace. Figure out what the user wants to do and update the name/description appropriately using the `update_taskspace` tool."
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
        
        // Launch VSCode in the cloned repository directory
        launchVSCode(for: taskspace, in: project.directoryPath)
        
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

// MARK: - IpcMessageDelegate

extension ProjectManager {
    
    func handleGetTaskspaceState(_ payload: GetTaskspaceStatePayload, messageId: String) async -> MessageHandlingResult<TaskspaceStateResponse> {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for get_taskspace_state")
            return .notForMe
        }
        
        // Look for taskspace with matching UUID in current project
        guard let taskspace = currentProject.findTaskspace(uuid: payload.taskspaceUuid) else {
            Logger.shared.log("ProjectManager: Taskspace \(payload.taskspaceUuid) not found in project \(currentProject.name)")
            return .notForMe
        }
        
        Logger.shared.log("ProjectManager: Found taskspace \(taskspace.name) for UUID: \(payload.taskspaceUuid)")
        
        // Get agent command based on taskspace state and selected agent
        guard let agentCommand = agentManager.getAgentCommand(for: taskspace, selectedAgent: selectedAgent) else {
            Logger.shared.log("ProjectManager: No valid agent command for taskspace \(taskspace.name)")
            return .notForMe
        }
        
        // Determine if agent should launch based on taskspace state
        // For now, always launch since we don't have a complete state
        let shouldLaunch = true
        
        let response = TaskspaceStateResponse(
            agentCommand: agentCommand,
            shouldLaunch: shouldLaunch
        )
        
        Logger.shared.log("ProjectManager: Responding with shouldLaunch=\(shouldLaunch), command=\(agentCommand)")
        return .handled(response)
    }
    
    func handleSpawnTaskspace(_ payload: SpawnTaskspacePayload, messageId: String) async -> MessageHandlingResult<SpawnTaskspaceResponse> {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for spawn_taskspace")
            return .notForMe
        }
        
        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log("ProjectManager: Project path mismatch: \(payload.projectPath) != \(currentProject.directoryPath)")
            return .notForMe
        }
        
        Logger.shared.log("ProjectManager: Creating taskspace \(payload.name) (parent UUID: \(payload.taskspaceUuid))")
        
        do {
            // Create new taskspace with fresh UUID
            let taskspace = Taskspace(
                name: payload.name,
                description: payload.taskDescription,
                initialPrompt: payload.initialPrompt
            )
            
            Logger.shared.log("ProjectManager: Created new taskspace with UUID: \(taskspace.id.uuidString)")
            
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
    
    func handleLogProgress(_ payload: LogProgressPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for log_progress")
            return .notForMe
        }
        
        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log("ProjectManager: Project path mismatch for log_progress: \(payload.projectPath) != \(currentProject.directoryPath)")
            return .notForMe
        }
        
        // Find taskspace by UUID
        guard let taskspaceIndex = currentProject.findTaskspaceIndex(uuid: payload.taskspaceUuid) else {
            Logger.shared.log("ProjectManager: Taskspace \(payload.taskspaceUuid) not found for log_progress")
            return .notForMe
        }
        
        Logger.shared.log("ProjectManager: Adding log to taskspace \(payload.taskspaceUuid): \(payload.message)")
        
        do {
            // Create log entry
            let logCategory = LogCategory(rawValue: payload.category) ?? .info
            let logEntry = TaskspaceLog(message: payload.message, category: logCategory)
            
            // Update taskspace with new log
            var updatedProject = currentProject
            updatedProject.taskspaces[taskspaceIndex].logs.append(logEntry)
            
            // Save updated taskspace
            try updatedProject.taskspaces[taskspaceIndex].save(in: currentProject.directoryPath)
            
            // Update UI
            DispatchQueue.main.async {
                self.currentProject = updatedProject
                Logger.shared.log("ProjectManager: Updated taskspace logs")
            }
            
            // Transition from Hatchling state if needed
            transitionFromHatchlingIfNeeded(taskspaceUuid: payload.taskspaceUuid)
            
            return .handled(EmptyResponse())
            
        } catch {
            Logger.shared.log("ProjectManager: Failed to save log entry: \(error)")
            return .notForMe
        }
    }
    
    func handleSignalUser(_ payload: SignalUserPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        guard let currentProject = currentProject else {
            Logger.shared.log("ProjectManager: No current project for signal_user")
            return .notForMe
        }
        
        // Check if this project path matches our current project
        guard currentProject.directoryPath == payload.projectPath else {
            Logger.shared.log("ProjectManager: Project path mismatch for signal_user: \(payload.projectPath) != \(currentProject.directoryPath)")
            return .notForMe
        }
        
        // Find taskspace by UUID
        guard let taskspaceIndex = currentProject.findTaskspaceIndex(uuid: payload.taskspaceUuid) else {
            Logger.shared.log("ProjectManager: Taskspace \(payload.taskspaceUuid) not found for signal_user")
            return .notForMe
        }
        
        Logger.shared.log("ProjectManager: Signaling user for taskspace \(payload.taskspaceUuid): \(payload.message)")
        
        do {
            // Update taskspace with signal log entry
            var updatedProject = currentProject
            
            // Add a log entry to indicate user attention is needed
            let signalLog = TaskspaceLog(message: payload.message, category: .question)
            updatedProject.taskspaces[taskspaceIndex].logs.append(signalLog)
            
            // Save updated taskspace
            try updatedProject.taskspaces[taskspaceIndex].save(in: currentProject.directoryPath)
            
            // Update UI and dock badge
            DispatchQueue.main.async {
                self.currentProject = updatedProject
                
                // TODO: Update dock badge count
                // TODO: Bring app to foreground or show notification
                
                Logger.shared.log("ProjectManager: Added signal log for user attention")
            }
            
            // Transition from Hatchling state if needed
            transitionFromHatchlingIfNeeded(taskspaceUuid: payload.taskspaceUuid)
            
            return .handled(EmptyResponse())
            
        } catch {
            Logger.shared.log("ProjectManager: Failed to update taskspace attention: \(error)")
            return .notForMe
        }
    }
    
    func handleUpdateTaskspace(_ payload: UpdateTaskspacePayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        guard let project = currentProject else {
            return .notForMe
        }
        
        // Find the taskspace by UUID
        guard let taskspaceIndex = project.taskspaces.firstIndex(where: { $0.id.uuidString.lowercased() == payload.taskspaceUuid.lowercased() }) else {
            Logger.shared.log("ProjectManager: Taskspace not found for UUID: \(payload.taskspaceUuid)")
            return .notForMe
        }
        
        var updatedProject = project
        updatedProject.taskspaces[taskspaceIndex].name = payload.name
        updatedProject.taskspaces[taskspaceIndex].description = payload.description
        
        // Update UI
        DispatchQueue.main.async {
            self.currentProject = updatedProject
            Logger.shared.log("ProjectManager: Updated taskspace: \(payload.name)")
        }
        
        // Transition from Hatchling state if needed
        transitionFromHatchlingIfNeeded(taskspaceUuid: payload.taskspaceUuid)
        
        return .handled(EmptyResponse())
    }
    
    /// Transitions a taskspace from Hatchling to Resume state if needed
    private func transitionFromHatchlingIfNeeded(taskspaceUuid: String) {
        guard let project = currentProject else { return }
        
        guard let taskspaceIndex = project.taskspaces.firstIndex(where: { $0.id.uuidString.lowercased() == taskspaceUuid.lowercased() }) else {
            return
        }
        
        // Only transition if currently in Hatchling state
        if case .hatchling = project.taskspaces[taskspaceIndex].state {
            var updatedProject = project
            updatedProject.taskspaces[taskspaceIndex].state = .resume
            
            DispatchQueue.main.async {
                self.currentProject = updatedProject
                Logger.shared.log("ProjectManager: Transitioned taskspace \(taskspaceUuid) from Hatchling to Resume")
            }
        }
    }
    
    /// Launch VSCode for a taskspace directory
    private func launchVSCode(for taskspace: Taskspace, in projectPath: String) {
        let taskspaceDir = taskspace.directoryPath(in: projectPath)
        
        let vscodeProcess = Process()
        vscodeProcess.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        vscodeProcess.arguments = ["-a", "Visual Studio Code", taskspaceDir]
        
        do {
            try vscodeProcess.run()
            Logger.shared.log("ProjectManager: Launched VSCode for taskspace: \(taskspace.name)")
        } catch {
            Logger.shared.log("ProjectManager: Failed to launch VSCode for \(taskspace.name): \(error)")
        }
    }
}
