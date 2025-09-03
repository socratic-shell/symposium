import Foundation

/// Represents a Symposium project containing multiple taskspaces
struct Project: Codable, Identifiable {
    let id: UUID
    let name: String
    let gitURL: String
    let directoryPath: String
    var taskspaces: [Taskspace] = []
    let createdAt: Date
    
    init(name: String, gitURL: String, directoryPath: String) {
        self.id = UUID()
        self.name = name
        self.gitURL = gitURL
        self.directoryPath = directoryPath
        self.createdAt = Date()
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
        
        return try decoder.decode(Project.self, from: data)
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
