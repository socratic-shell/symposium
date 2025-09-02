import Foundation
import Combine

class DaemonManager: ObservableObject {
    @Published var isConnected = false
    @Published var error: String?
    
    private var clientProcess: Process?
    
    func startClient(mcpServerPath: String) {
        guard clientProcess == nil else { return }
        
        error = nil
        
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
        
        // Capture output for debugging
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        
        do {
            try process.run()
            self.clientProcess = process
            
            DispatchQueue.main.async {
                self.isConnected = true
            }
            
            // Monitor process termination
            process.waitUntilExit()
            
            DispatchQueue.main.async {
                self.isConnected = false
                if process.terminationStatus != 0 {
                    self.error = "Client exited with status \(process.terminationStatus)"
                }
            }
            
        } catch {
            DispatchQueue.main.async {
                self.error = "Failed to start client: \(error.localizedDescription)"
            }
        }
    }
}
