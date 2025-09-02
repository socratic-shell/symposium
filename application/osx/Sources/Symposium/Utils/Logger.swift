import Foundation

class Logger {
    static let shared = Logger()
    private let logFile: URL
    
    private init() {
        let documentsPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        logFile = documentsPath.appendingPathComponent("symposium_debug.log")
        
        // Clear log on startup
        let startMessage = "=== Symposium Debug Log Started at \(Date()) ===\n"
        try? startMessage.write(to: logFile, atomically: true, encoding: .utf8)
        print("SYMPOSIUM Logger: Log file at \(logFile.path)")
    }
    
    func log(_ message: String) {
        let logMessage = "[\(Date())] \(message)\n"
        
        // Also print to console immediately
        print("SYMPOSIUM: \(message)")
        
        // Write to file
        do {
            if FileManager.default.fileExists(atPath: logFile.path) {
                let fileHandle = try FileHandle(forWritingTo: logFile)
                fileHandle.seekToEndOfFile()
                fileHandle.write(logMessage.data(using: .utf8)!)
                fileHandle.closeFile()
            } else {
                try logMessage.write(to: logFile, atomically: true, encoding: .utf8)
            }
        } catch {
            print("SYMPOSIUM Logger ERROR: \(error)")
        }
    }
}
