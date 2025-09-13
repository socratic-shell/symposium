import Foundation

/// Represents a taskspace within a project
struct Taskspace: Codable, Identifiable {
    let id: UUID
    var name: String
    var description: String
    var state: TaskspaceState
    var logs: [TaskspaceLog] = []
    var vscodeWindowID: Int? = nil
    let createdAt: Date
    
    /// Timestamp of last screenshot capture (not persisted, transient UI state)
    var lastScreenshotAt: Date?
    
    /// Flag to trigger deletion confirmation dialog (not persisted, transient UI state)
    var pendingDeletion: Bool = false
    
    private enum CodingKeys: String, CodingKey {
        case id, name, description, state, logs, vscodeWindowID, createdAt
    }
    
    init(name: String, description: String, initialPrompt: String? = nil) {
        self.id = UUID()
        self.name = name
        self.description = description
        self.state = initialPrompt != nil ? .hatchling(initialPrompt: initialPrompt!) : .resume
        self.createdAt = Date()
    }
    
    /// Directory path for this taskspace within project
    func directoryPath(in projectPath: String) -> String {
        return "\(projectPath)/task-\(id.uuidString)"
    }
    
    /// Get the initial prompt if taskspace is in hatchling state
    var initialPrompt: String? {
        switch state {
        case .hatchling(let prompt):
            return prompt
        case .resume:
            return nil
        }
    }
    
    /// Path to taskspace.json file
    func taskspaceFilePath(in projectPath: String) -> String {
        return "\(directoryPath(in: projectPath))/taskspace.json"
    }
    
    /// Add a log entry to this taskspace
    mutating func addLog(_ log: TaskspaceLog) {
        logs.append(log)
    }
    
    /// Check if taskspace needs user attention
    var needsAttention: Bool {
        return logs.contains { $0.category == .question }
    }
    
    /// Save taskspace metadata to taskspace.json
    func save(in projectPath: String) throws {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = .prettyPrinted
        
        let data = try encoder.encode(self)
        let filePath = taskspaceFilePath(in: projectPath)
        try data.write(to: URL(fileURLWithPath: filePath))
    }
    
    /// Load taskspace from taskspace.json file
    static func load(from filePath: String) throws -> Taskspace {
        let data = try Data(contentsOf: URL(fileURLWithPath: filePath))
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        
        return try decoder.decode(Taskspace.self, from: data)
    }
}

/// State of a taskspace
enum TaskspaceState: Codable {
    case hatchling(initialPrompt: String)  // Not started yet, has initial prompt
    case resume                            // Active, should resume from where it left off
}

/// Log entry for taskspace progress
struct TaskspaceLog: Codable, Identifiable {
    let id: UUID
    let message: String
    let category: LogCategory
    let timestamp: Date
    
    init(message: String, category: LogCategory) {
        self.id = UUID()
        self.message = message
        self.category = category
        self.timestamp = Date()
    }
}

/// Categories for log messages with visual indicators
enum LogCategory: String, Codable, CaseIterable {
    case info = "info"           // ℹ️
    case warn = "warn"           // ⚠️
    case error = "error"         // ❌
    case milestone = "milestone" // ✅
    case question = "question"   // ❓
    
    var icon: String {
        switch self {
        case .info: return "ℹ️"
        case .warn: return "⚠️"
        case .error: return "❌"
        case .milestone: return "✅"
        case .question: return "❓"
        }
    }
}
