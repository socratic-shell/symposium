import Foundation
import SwiftUI
import os.log

class Logger: ObservableObject {
    static let shared = Logger()
    @Published var logs: [String] = []
    private let maxLogLines = 1024
    
    // Reference to IpcManager for sending logs to daemon
    private weak var ipcManager: IpcManager?
    
    // System log for Console.app visibility
    private static let osLog = OSLog(subsystem: "com.symposium.app", category: "general")
    
    private init() {
        let startMessage = "=== Symposium Debug Log Started at \(Date()) ==="
        logs.append(startMessage)
    }
    
    /// Set the IPC manager for sending logs to daemon
    func setIpcManager(_ manager: IpcManager) {
        self.ipcManager = manager
    }
    
    private lazy var dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss.SSS"
        return formatter
    }()
    
    func log(_ message: String, level: String = "info") {
        osLog(message, level: level)

        let timestamp = dateFormatter.string(from: Date())
        let logMessage = "[\(timestamp)] \(message)"
        
        DispatchQueue.main.async {
            self.logs.append(logMessage)
            
            // Keep only the last 1024 lines
            if self.logs.count > self.maxLogLines {
                self.logs.removeFirst(self.logs.count - self.maxLogLines)
            }
        }
        
        // Send to daemon if IPC manager is available
        if let ipcManager = self.ipcManager {
            let daemonLogMessage = LogMessage(level: level, message: "[APP:\(ProcessInfo.processInfo.processIdentifier)] \(message)")
            ipcManager.sendBroadcastMessage(type: "log", payload: daemonLogMessage)
        }
    }
    
    /// Local-only logging that never broadcasts (safe to use from IPC code)
    func osLog(_ message: String, level: String = "info") {        
        let timestamp = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
        let processId = ProcessInfo.processInfo.processIdentifier
        let logMessage = "[\(timestamp)] [\(processId)] [\(level)] \(message)"

        // Send to system log (visible in Console.app)
        let osLogType: OSLogType = switch level {
        case "debug": .debug
        case "error": .error
        default: .info
        }
        os_log("%{public}@", log: Logger.osLog, type: osLogType, logMessage)

    }
    
    // Convenience methods for different log levels
    func debug(_ message: String) {
        log(message, level: "debug")
    }
    
    func info(_ message: String) {
        log(message, level: "info")
    }
    
    func error(_ message: String) {
        log(message, level: "error")
    }
}
