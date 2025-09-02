import Foundation
import AppKit

class AgentManager: ObservableObject {
    @Published var availableAgents: [AgentInfo] = []
    @Published var isScanning = false
    
    init() {
        scanForAgents()
    }
    
    func scanForAgents() {
        isScanning = true
        
        DispatchQueue.global(qos: .userInitiated).async {
            var agents: [AgentInfo] = []
            
            // Check for Q CLI
            if let qcliInfo = self.detectQCLI() {
                agents.append(qcliInfo)
            }
            
            // Check for Claude Code
            if let claudeInfo = self.detectClaudeCode() {
                agents.append(claudeInfo)
            }
            
            DispatchQueue.main.async {
                self.availableAgents = agents
                self.isScanning = false
            }
        }
    }
    
    private func detectQCLI() -> AgentInfo? {
        // Check if q command exists in PATH
        let qPath = findExecutable(name: "q")
        guard let path = qPath else { return nil }
        
        // Verify it's actually Q CLI by checking version
        let version = getQCLIVersion(path: path)
        
        // Check if MCP is configured
        let mcpConfigured = checkQCLIMCPConfiguration()
        
        return AgentInfo(
            id: "qcli",
            name: "Q CLI",
            description: "Amazon Q Developer CLI",
            executablePath: path,
            version: version,
            isInstalled: true,
            isMCPConfigured: mcpConfigured
        )
    }
    
    private func detectClaudeCode() -> AgentInfo? {
        // First try to find claude in PATH
        if let path = findExecutable(name: "claude") {
            let version = getClaudeCodeVersion(path: path)
            let mcpConfigured = checkClaudeCodeMCPConfiguration()
            
            return AgentInfo(
                id: "claude",
                name: "Claude Code",
                description: "Anthropic Claude for coding",
                executablePath: path,
                version: version,
                isInstalled: true,
                isMCPConfigured: mcpConfigured
            )
        }
        
        // If not found in PATH, check common installation paths
        let possiblePaths = [
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "~/.local/bin/claude",
            "~/.volta/bin/claude"
        ].map { NSString(string: $0).expandingTildeInPath }
        
        for path in possiblePaths {
            if FileManager.default.isExecutableFile(atPath: path) {
                let version = getClaudeCodeVersion(path: path)
                let mcpConfigured = checkClaudeCodeMCPConfiguration()
                
                return AgentInfo(
                    id: "claude",
                    name: "Claude Code",
                    description: "Anthropic Claude for coding",
                    executablePath: path,
                    version: version,
                    isInstalled: true,
                    isMCPConfigured: mcpConfigured
                )
            }
        }
        
        // Return not installed info
        return AgentInfo(
            id: "claude",
            name: "Claude Code",
            description: "Anthropic Claude for coding",
            executablePath: nil,
            version: nil,
            isInstalled: false,
            isMCPConfigured: false
        )
    }
    
    private func findExecutable(name: String) -> String? {
        let process = Process()
        process.launchPath = "/usr/bin/which"
        process.arguments = [name]
        
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()
        
        do {
            try process.run()
            process.waitUntilExit()
            
            if process.terminationStatus == 0 {
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
                return output?.isEmpty == false ? output : nil
            }
        } catch {
            print("Error finding executable \(name): \(error)")
        }
        
        return nil
    }
    
    private func getQCLIVersion(path: String) -> String? {
        return runCommand(path: path, arguments: ["--version"])
    }
    
    private func getClaudeCodeVersion(path: String) -> String? {
        return runCommand(path: path, arguments: ["--version"])
    }
    
    private func runCommand(path: String, arguments: [String]) -> String? {
        let process = Process()
        process.launchPath = path
        process.arguments = arguments
        
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()
        
        do {
            try process.run()
            process.waitUntilExit()
            
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
            return output?.isEmpty == false ? output : nil
        } catch {
            print("Error running command \(path): \(error)")
            return nil
        }
    }
    
    private func checkQCLIMCPConfiguration() -> Bool {
        // For now, assume MCP is configured if Q CLI is installed
        // TODO: Implement proper MCP configuration checking
        return true
    }
    
    private func checkClaudeCodeMCPConfiguration() -> Bool {
        // For now, assume MCP is configured if Claude Code is installed  
        // TODO: Implement proper MCP configuration checking
        return true
    }
    
    private func readMCPConfig(path: String) -> String? {
        guard FileManager.default.fileExists(atPath: path) else { return nil }
        
        do {
            return try String(contentsOfFile: path, encoding: .utf8)
        } catch {
            print("Error reading MCP config at \(path): \(error)")
            return nil
        }
    }
}

struct AgentInfo: Identifiable {
    let id: String
    let name: String
    let description: String
    let executablePath: String?
    let version: String?
    let isInstalled: Bool
    let isMCPConfigured: Bool
    
    var statusText: String {
        if !isInstalled {
            return "Not installed"
        } else if !isMCPConfigured {
            return "MCP not configured"
        } else {
            return "Ready"
        }
    }
    
    var statusColor: NSColor {
        if !isInstalled {
            return .systemRed
        } else if !isMCPConfigured {
            return .systemOrange
        } else {
            return .systemGreen
        }
    }
    
    var statusIcon: String {
        if !isInstalled {
            return "xmark.circle.fill"
        } else if !isMCPConfigured {
            return "exclamationmark.triangle.fill"
        } else {
            return "checkmark.circle.fill"
        }
    }
}
