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
    
    func log(_ message: String) {
        let logMessage = "[\(Date())] \(message)"
        
        DispatchQueue.main.async {
            self.logs.append(logMessage)
            
            // Keep only the last 1024 lines
            if self.logs.count > self.maxLogLines {
                self.logs.removeFirst(self.logs.count - self.maxLogLines)
            }
        }
    }
}
