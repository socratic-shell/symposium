import Foundation

/// Manages project creation, loading, and operations
class ProjectManager: ObservableObject, IpcMessageDelegate {
    @Published var currentProject: Project?
    @Published var isLoading = false
    @Published var errorMessage: String?
    
    private let ipcManager = IpcManager()
    private let agentManager: AgentManager
    private let selectedAgent: String
    
    var mcpStatus: IpcManager { ipcManager }
    
    init(agentManager: AgentManager, selectedAgent: String) {
        self.agentManager = agentManager
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
            
            // Register as IPC delegate for this project
            self.ipcManager.addDelegate(self)
            Logger.shared.log("ProjectManager: Registered as IPC delegate for project: \(project.name)")
            
            self.startMCPClient()
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
        // TODO: Look up taskspace by UUID in current project
        // TODO: Determine agent command based on user preferences and taskspace state
        // TODO: Check if taskspace exists and is not complete
        
        Logger.shared.log("ProjectManager: TODO - Handle get_taskspace_state for UUID: \(payload.taskspaceUuid)")
        return .notForMe
    }
    
    func handleSpawnTaskspace(_ payload: SpawnTaskspacePayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        // TODO: Check if this project path matches our current project
        // TODO: Create taskspace directory, clone repo, save metadata
        // TODO: Update UI with new taskspace
        
        Logger.shared.log("ProjectManager: TODO - Handle spawn_taskspace: \(payload.name) in \(payload.projectPath)")
        return .notForMe
    }
    
    func handleLogProgress(_ payload: LogProgressPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        // TODO: Find taskspace by UUID in current project
        // TODO: Add log entry to taskspace and save to taskspace.json
        // TODO: Update UI to show new log
        
        Logger.shared.log("ProjectManager: TODO - Handle log_progress for \(payload.taskspaceUuid): \(payload.message)")
        return .notForMe
    }
    
    func handleSignalUser(_ payload: SignalUserPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse> {
        // TODO: Find taskspace by UUID in current project
        // TODO: Set taskspace attention flag and update dock badge
        // TODO: Update UI to highlight taskspace
        
        Logger.shared.log("ProjectManager: TODO - Handle signal_user for \(payload.taskspaceUuid): \(payload.message)")
        return .notForMe
    }
}
