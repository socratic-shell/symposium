import Foundation
import Combine

class DaemonManager: ObservableObject {
    @Published var isConnected = false
    @Published var error: String?
    @Published var debugOutput = ""
    
    private var clientProcess: Process?
    
    func startClient(mcpServerPath: String) {
        guard clientProcess == nil else { return }
        
        error = nil
        debugOutput = "Starting symposium-mcp client...\n"
        debugOutput += "Path: \(mcpServerPath)\n"
        debugOutput += "Command: \(mcpServerPath) client\n"
        
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
                self.debugOutput += "Client process started successfully\n"
            }
            
            // Read output in background
            DispatchQueue.global(qos: .background).async {
                let data = outputPipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8) ?? ""
                
                DispatchQueue.main.async {
                    self.debugOutput += "Output: \(output)\n"
                }
            }
            
            // Monitor process termination
            process.waitUntilExit()
            
            DispatchQueue.main.async {
                self.isConnected = false
                self.debugOutput += "Client process exited with status \(process.terminationStatus)\n"
                if process.terminationStatus != 0 {
                    self.error = "Client exited with status \(process.terminationStatus)"
                }
            }
            
        } catch {
            DispatchQueue.main.async {
                self.error = "Failed to start client: \(error.localizedDescription)"
                self.debugOutput += "Error: \(error.localizedDescription)\n"
            }
        }
    }
}
