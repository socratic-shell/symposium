import Foundation

/// Errors that can occur during project validation
enum ProjectValidationError: LocalizedError {
    case missingProjectFile
    case invalidProjectStructure
    case directoryNotAccessible
    case unknownValidationFailure
    
    var errorDescription: String? {
        switch self {
        case .missingProjectFile:
            return "This directory is not a valid Symposium project"
        case .invalidProjectStructure:
            return "The project structure is invalid or corrupted"
        case .directoryNotAccessible:
            return "Unable to access the selected directory"
        case .unknownValidationFailure:
            return "An unknown error occurred while validating the project"
        }
    }
    
    var recoverySuggestion: String? {
        switch self {
        case .missingProjectFile:
            return "Please select a directory that contains a project.json file, or create a new project instead."
        case .invalidProjectStructure:
            return "Try selecting a different project directory or create a new project."
        case .directoryNotAccessible:
            return "Check that you have permission to access this directory and try again."
        case .unknownValidationFailure:
            return "Please try selecting a different directory or create a new project."
        }
    }
}

/// Version 0 project structure for backward compatibility
private struct ProjectV0: Codable {
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    var taskspaces: [Taskspace]
    let createdAt: Date
}

/// Represents a Symposium project containing multiple taskspaces
struct Project: Codable, Identifiable {
    
    /// Computed property for repository path
    var repoPath: String {
        return "\(directoryPath)/.git"
    }
    
    let version: Int
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    let agent: String?
    let defaultBranch: String?
    let remoteName: String
    var taskspaces: [Taskspace] = []
    let createdAt: Date
    var stackedWindowsEnabled: Bool = false
    
    init(name: String, gitURL: String, directoryPath: String, agent: String? = nil, defaultBranch: String? = nil, remoteName: String = "origin") {
        self.version = 2
        self.id = UUID()
        self.name = name
        self.gitURL = gitURL
        self.directoryPath = directoryPath
        self.agent = agent
        self.defaultBranch = defaultBranch
        self.remoteName = remoteName
        self.createdAt = Date()
        self.stackedWindowsEnabled = false
    }
    
    // Internal initializer for migration
    private init(version: Int, id: UUID, name: String, gitURL: String, directoryPath: String, agent: String?, defaultBranch: String?, remoteName: String, taskspaces: [Taskspace], createdAt: Date, stackedWindowsEnabled: Bool = false) {
        self.version = version
        self.id = id
        self.name = name
        self.gitURL = gitURL
        self.directoryPath = directoryPath
        self.agent = agent
        self.defaultBranch = defaultBranch
        self.remoteName = remoteName
        self.taskspaces = taskspaces
        self.createdAt = createdAt
        self.stackedWindowsEnabled = stackedWindowsEnabled
    }
    
    /// Path to project.json file
    var projectFilePath: String {
        return "\(directoryPath)/project.json"
    }
    
    /// Save project metadata to project.json
    func save() throws {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = .prettyPrinted
        
        let data = try encoder.encode(self)
        try data.write(to: URL(fileURLWithPath: projectFilePath))
    }
    
    /// Load project from project.json file
    static func load(from directoryPath: String) throws -> Project {
        let projectFilePath = "\(directoryPath)/project.json"
        let data = try Data(contentsOf: URL(fileURLWithPath: projectFilePath))
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        
        do {
            // Try to decode with current schema
            var project = try decoder.decode(Project.self, from: data)
            
            // Handle version migration
            if project.version == 1 {
                // Migrate from version 1 to version 2
                project = Project(
                    version: 2,
                    id: project.id,
                    name: project.name,
                    gitURL: project.gitURL,
                    directoryPath: project.directoryPath,
                    agent: project.agent,
                    defaultBranch: project.defaultBranch,
                    remoteName: "origin",
                    taskspaces: project.taskspaces,
                    createdAt: project.createdAt,
                    stackedWindowsEnabled: false
                )
                
                // Save migrated project back to disk
                try project.save()
            }
            
            return project
        } catch {
            // Fall back to legacy schema (version 0) and migrate
            let legacyProject = try decoder.decode(ProjectV0.self, from: data)
            let migratedProject = Project(
                version: 2,
                id: legacyProject.id,
                name: legacyProject.name,
                gitURL: legacyProject.gitURL,
                directoryPath: legacyProject.directoryPath,
                agent: nil,
                defaultBranch: nil,
                remoteName: "origin",
                taskspaces: legacyProject.taskspaces,
                createdAt: legacyProject.createdAt,
                stackedWindowsEnabled: false
            )
            
            // Save migrated project back to disk
            try migratedProject.save()
            
            return migratedProject
        }
    }
    
    /// Check if directory contains a valid Symposium project
    static func isValidProjectDirectory(_ path: String) -> Bool {
        let projectFilePath = "\(path)/project.json"
        return FileManager.default.fileExists(atPath: projectFilePath)
    }
    
    /// Validate project directory with detailed error reporting
    static func validateProjectDirectory(_ path: String) -> Result<Void, ProjectValidationError> {
        // Check if directory is accessible
        guard FileManager.default.fileExists(atPath: path) else {
            return .failure(.directoryNotAccessible)
        }
        
        let projectFilePath = "\(path)/project.json"
        
        // Check if project.json exists
        guard FileManager.default.fileExists(atPath: projectFilePath) else {
            return .failure(.missingProjectFile)
        }
        
        // Attempt to load and validate project structure
        do {
            _ = try Project.load(from: path)
            return .success(())
        } catch {
            return .failure(.invalidProjectStructure)
        }
    }
    
    /// Find taskspace by UUID
    func findTaskspace(uuid: String) -> Taskspace? {
        return taskspaces.first { $0.id.uuidString.lowercased() == uuid.lowercased() }
    }
    
    /// Find taskspace index by UUID
    func findTaskspaceIndex(uuid: String) -> Int? {
        return taskspaces.firstIndex { $0.id.uuidString.lowercased() == uuid.lowercased() }
    }
    
    /// Reorder taskspaces by most recently activated (most recent first)
    mutating func reorderTaskspacesByActivation() {
        taskspaces.sort { $0.lastActivatedAt > $1.lastActivatedAt }
    }
    
    /// Update taskspace activation time and reorder
    mutating func activateTaskspace(uuid: String) {
        guard let index = findTaskspaceIndex(uuid: uuid) else { return }
        taskspaces[index].updateActivationTime()
        reorderTaskspacesByActivation()
    }
}
