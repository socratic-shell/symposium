import Foundation
import SwiftUI

class Logger: ObservableObject {
    static let shared = Logger()
    @Published var logs: [String] = []
    private let maxLogLines = 1024
    
    private init() {
        let startMessage = "=== Symposium Debug Log Started at \(Date()) ==="
        logs.append(startMessage)
    }
    
    private lazy var dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss.SSS"
        return formatter
    }()
    
    func log(_ message: String) {
        let timestamp = dateFormatter.string(from: Date())
        let logMessage = "[\(timestamp)] \(message)"
        
        DispatchQueue.main.async {
            self.logs.append(logMessage)
            
            // Keep only the last 1024 lines
            if self.logs.count > self.maxLogLines {
                self.logs.removeFirst(self.logs.count - self.maxLogLines)
            }
        }
    }
}
