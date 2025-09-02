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
        
        // Check if MCP is configured and get the path
        let (mcpConfigured, mcpPath) = checkQCLIMCPConfiguration(qPath: path)
        
        return AgentInfo(
            id: "qcli",
            name: "Q CLI",
            description: "Amazon Q Developer CLI",
            executablePath: path,
            version: version,
            isInstalled: true,
            isMCPConfigured: mcpConfigured,
            mcpServerPath: mcpPath
        )
    }
    
    private func detectClaudeCode() -> AgentInfo? {
        // First try to find claude in PATH
        if let path = findExecutable(name: "claude") {
            let version = getClaudeCodeVersion(path: path)
            let (mcpConfigured, mcpPath) = checkClaudeCodeMCPConfiguration(claudePath: path)
            
            return AgentInfo(
                id: "claude",
                name: "Claude Code",
                description: "Anthropic Claude for coding",
                executablePath: path,
                version: version,
                isInstalled: true,
                isMCPConfigured: mcpConfigured,
                mcpServerPath: mcpPath
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
                let (mcpConfigured, mcpPath) = checkClaudeCodeMCPConfiguration(claudePath: path)
                
                return AgentInfo(
                    id: "claude",
                    name: "Claude Code",
                    description: "Anthropic Claude for coding",
                    executablePath: path,
                    version: version,
                    isInstalled: true,
                    isMCPConfigured: mcpConfigured,
                    mcpServerPath: mcpPath
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
            isMCPConfigured: false,
            mcpServerPath: nil
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
        
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        
        do {
            try process.run()
            process.waitUntilExit()
            
            // Try stdout first
            let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
            let stdoutOutput = String(data: stdoutData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
            
            if let stdout = stdoutOutput, !stdout.isEmpty {
                return stdout
            }
            
            // If stdout is empty, try stderr (Q CLI outputs to stderr)
            let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
            let stderrOutput = String(data: stderrData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
            
            return stderrOutput?.isEmpty == false ? stderrOutput : nil
        } catch {
            print("Error running command \(path): \(error)")
            return nil
        }
    }
    
    private func checkQCLIMCPConfiguration(qPath: String) -> (Bool, String?) {
        // Use Q CLI's built-in MCP status command to check for symposium-mcp
        let output = runCommand(path: qPath, arguments: ["mcp", "status", "--name", "symposium"])
        
        guard let output = output, !output.isEmpty else {
            return (false, nil)
        }
        
        // Parse the output to extract the Command path
        // Look for lines like "Command : /path/to/symposium-mcp"
        let lines = output.components(separatedBy: .newlines)
        for line in lines {
            if line.contains("Command :") {
                let parts = line.components(separatedBy: ":")
                if parts.count >= 2 {
                    let path = parts[1].trimmingCharacters(in: .whitespaces)
                    return (true, path)
                }
            }
        }
        
        // Found output but couldn't parse path
        return (true, nil)
    }
    
    private func checkClaudeCodeMCPConfiguration(claudePath: String) -> (Bool, String?) {
        // Use Claude Code's built-in MCP list command to check for symposium-mcp
        let output = runCommand(path: claudePath, arguments: ["mcp", "list"])
        
        guard let output = output, !output.isEmpty else {
            return (false, nil)
        }
        
        // Parse the output to find symposium entry
        // Look for lines like "symposium: /path/to/symposium-mcp --dev-log - ✓ Connected"
        let lines = output.components(separatedBy: .newlines)
        for line in lines {
            if line.contains("symposium:") && line.contains("✓ Connected") {
                // Extract the path between "symposium: " and " --dev-log"
                let parts = line.components(separatedBy: ":")
                if parts.count >= 2 {
                    let pathPart = parts[1].trimmingCharacters(in: .whitespaces)
                    // Split by " --" to get just the path
                    let pathComponents = pathPart.components(separatedBy: " --")
                    if let path = pathComponents.first?.trimmingCharacters(in: .whitespaces) {
                        return (true, path)
                    }
                }
            }
        }
        
        // Check if symposium is listed but not connected
        for line in lines {
            if line.contains("symposium:") {
                return (false, nil) // Found but not connected
            }
        }
        
        return (false, nil)
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
    let mcpServerPath: String?
    
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
