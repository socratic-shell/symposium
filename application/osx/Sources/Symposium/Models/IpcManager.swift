import Foundation
import Combine

class IpcManager: ObservableObject {
    @Published var isConnected = false
    @Published var error: String?
    
    private var clientProcess: Process?
    
    func startClient(mcpServerPath: String) {
        guard clientProcess == nil else { return }
        
        error = nil
        Logger.shared.log("IpcManager: Starting symposium-mcp client...")
        Logger.shared.log("IpcManager: Path: \(mcpServerPath)")
        Logger.shared.log("IpcManager: Command: \(mcpServerPath) client")
        
        DispatchQueue.global(qos: .userInitiated).async {
            self.launchClient(mcpServerPath: mcpServerPath)
        }
    }
    
    func stopClient() {
        clientProcess?.terminate()
        clientProcess = nil
        
        DispatchQueue.main.async {
            self.isConnected = false
        }
    }
    
    private func launchClient(mcpServerPath: String) {
        let process = Process()
        process.launchPath = mcpServerPath
        process.arguments = ["client"]
        
        // Set up pipes for stdin/stdout/stderr
        let inputPipe = Pipe()
        let outputPipe = Pipe()
        
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = outputPipe
        
        do {
            try process.run()
            self.clientProcess = process
            
            DispatchQueue.main.async {
                self.isConnected = true
                Logger.shared.log("IpcManager: Client process started successfully")
                Logger.shared.log("IpcManager: isConnected set to \(self.isConnected)")
            }
            
            // Read output in background
            DispatchQueue.global(qos: .background).async {
                let data = outputPipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8) ?? ""
                
                DispatchQueue.main.async {
                    Logger.shared.log("IpcManager: Client output: \(output)")
                }
            }
            
            // Monitor process termination
            process.waitUntilExit()
            
            DispatchQueue.main.async {
                self.isConnected = false
                Logger.shared.log("IpcManager: Client process exited with status \(process.terminationStatus)")
                if process.terminationStatus != 0 {
                    self.error = "Client exited with status \(process.terminationStatus)"
                }
            }
            
        } catch {
            DispatchQueue.main.async {
                self.error = "Failed to start client: \(error.localizedDescription)"
                Logger.shared.log("IpcManager: Error starting client: \(error.localizedDescription)")
            }
        }
    }
}
