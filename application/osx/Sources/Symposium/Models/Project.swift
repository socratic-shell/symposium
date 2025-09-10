import Foundation

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
    let version: Int
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    let agent: String?
    var taskspaces: [Taskspace] = []
    let createdAt: Date
    
    init(name: String, gitURL: String, directoryPath: String, agent: String? = nil) {
        self.version = 1
        self.id = UUID()
        self.name = name
        self.gitURL = gitURL
        self.directoryPath = directoryPath
        self.agent = agent
        self.createdAt = Date()
    }
    
    // Internal initializer for migration
    private init(version: Int, id: UUID, name: String, gitURL: String, directoryPath: String, agent: String?, taskspaces: [Taskspace], createdAt: Date) {
        self.version = version
        self.id = id
        self.name = name
        self.gitURL = gitURL
        self.directoryPath = directoryPath
        self.agent = agent
        self.taskspaces = taskspaces
        self.createdAt = createdAt
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
            return try decoder.decode(Project.self, from: data)
        } catch {
            // Fall back to legacy schema and migrate
            let legacyProject = try decoder.decode(ProjectV0.self, from: data)
            let migratedProject = Project(
                version: 1,
                id: legacyProject.id,
                name: legacyProject.name,
                gitURL: legacyProject.gitURL,
                directoryPath: legacyProject.directoryPath,
                agent: nil,
                taskspaces: legacyProject.taskspaces,
                createdAt: legacyProject.createdAt
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
    
    /// Find taskspace by UUID
    func findTaskspace(uuid: String) -> Taskspace? {
        return taskspaces.first { $0.id.uuidString.lowercased() == uuid.lowercased() }
    }
    
    /// Find taskspace index by UUID
    func findTaskspaceIndex(uuid: String) -> Int? {
        return taskspaces.firstIndex { $0.id.uuidString.lowercased() == uuid.lowercased() }
    }
}
