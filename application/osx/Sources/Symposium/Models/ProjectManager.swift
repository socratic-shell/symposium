import Foundation

/// Manages project creation, loading, and operations
class ProjectManager: ObservableObject {
    @Published var currentProject: Project?
    @Published var isLoading = false
    @Published var errorMessage: String?
    
    private let daemonManager = DaemonManager()
    private var agentManager: AgentManager?
    private var selectedAgent: String = "qcli"
    
    var mcpStatus: DaemonManager { daemonManager }
    
    func configure(agentManager: AgentManager, selectedAgent: String) {
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
        DispatchQueue.main.async {
            self.currentProject = project
            self.errorMessage = nil
            self.startMCPClient()
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
        let project = try Project.load(from: directoryPath)
        
        // Load existing taskspaces
        var loadedProject = project
        loadedProject.taskspaces = try loadTaskspaces(from: directoryPath)
        
        // Set as current project
        DispatchQueue.main.async {
            self.currentProject = loadedProject
            self.errorMessage = nil
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
        stopMCPClient()
        DispatchQueue.main.async {
            self.currentProject = nil
            self.errorMessage = nil
        }
    }
    
    private func startMCPClient() {
        guard let agentManager = agentManager else { return }
        
        // Stop any existing client first
        daemonManager.stopClient()
        
        // Start client if we have a valid selected agent
        if let selectedAgentInfo = agentManager.availableAgents.first(where: { $0.id == selectedAgent }),
           selectedAgentInfo.isInstalled && selectedAgentInfo.isMCPConfigured,
           let mcpPath = selectedAgentInfo.mcpServerPath {
            daemonManager.startClient(mcpServerPath: mcpPath)
        }
    }
    
    private func stopMCPClient() {
        daemonManager.stopClient()
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
}

/// Errors that can occur during project operations
enum ProjectError: LocalizedError {
    case directoryAlreadyExists
    case invalidProjectDirectory
    case failedToCreateDirectory
    case failedToSaveProject
    
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
        }
    }
}
