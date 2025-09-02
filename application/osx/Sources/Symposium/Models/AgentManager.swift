import Foundation
import AppKit

class AgentManager: ObservableObject {
    @Published var availableAgents: [AgentInfo] = []
    @Published var isScanning = false
    @Published var debugOutput: String = ""
    
    init() {
        scanForAgents()
    }
    
    func scanForAgents() {
        isScanning = true
        debugOutput = "Starting agent scan...\n"
        
        DispatchQueue.global(qos: .userInitiated).async {
            var agents: [AgentInfo] = []
            
            // Check for Q CLI
            DispatchQueue.main.async {
                self.debugOutput += "Checking for Q CLI...\n"
            }
            if let qcliInfo = self.detectQCLI() {
                agents.append(qcliInfo)
                DispatchQueue.main.async {
                    self.debugOutput += "Q CLI detected: \(qcliInfo.statusText)\n"
                }
            } else {
                DispatchQueue.main.async {
                    self.debugOutput += "Q CLI not found\n"
                }
            }
            
            // Check for Claude Code
            DispatchQueue.main.async {
                self.debugOutput += "Checking for Claude Code...\n"
            }
            if let claudeInfo = self.detectClaudeCode() {
                agents.append(claudeInfo)
                DispatchQueue.main.async {
                    self.debugOutput += "Claude Code detected: \(claudeInfo.statusText)\n"
                }
            } else {
                DispatchQueue.main.async {
                    self.debugOutput += "Claude Code not found\n"
                }
            }
            
            DispatchQueue.main.async {
                self.availableAgents = agents
                self.isScanning = false
                self.debugOutput += "Scan complete. Found \(agents.count) agents.\n"
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
        DispatchQueue.main.async {
            self.debugOutput += "Looking for Claude Code executable...\n"
        }
        
        // First try to find claude in PATH
        if let path = findExecutable(name: "claude") {
            DispatchQueue.main.async {
                self.debugOutput += "Found claude at: \(path)\n"
            }
            let version = getClaudeCodeVersion(path: path)
            DispatchQueue.main.async {
                self.debugOutput += "Claude version: \(version ?? "unknown")\n"
            }
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
        
        DispatchQueue.main.async {
            self.debugOutput += "Claude not found in PATH, checking common locations...\n"
        }
        
        // If not found in PATH, check common installation paths
        let possiblePaths = [
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "~/.local/bin/claude",
            "~/.volta/bin/claude"
        ].map { NSString(string: $0).expandingTildeInPath }
        
        for path in possiblePaths {
            DispatchQueue.main.async {
                self.debugOutput += "Checking: \(path)\n"
            }
            if FileManager.default.isExecutableFile(atPath: path) {
                DispatchQueue.main.async {
                    self.debugOutput += "Found executable at: \(path)\n"
                }
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
        
        DispatchQueue.main.async {
            self.debugOutput += "Claude Code not found anywhere\n"
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
        
        DispatchQueue.main.async {
            self.debugOutput += "Claude MCP command: \(claudePath) mcp list\n"
            self.debugOutput += "Claude MCP output: \(output ?? "nil")\n\n"
        }
        
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
    
    /// Generate command for hatchling taskspace with initial prompt
    func getHatchlingCommand(initialPrompt: String) -> [String]? {
        guard isInstalled && isMCPConfigured else { return nil }
        
        switch id {
        case "qcli":
            return ["q", "chat", initialPrompt]
        case "claude-code":
            // TODO: Implement claude-code hatchling command
            return nil
        default:
            return nil
        }
    }
    
    /// Generate command for resume taskspace
    func getResumeCommand() -> [String]? {
        guard isInstalled && isMCPConfigured else { return nil }
        
        switch id {
        case "qcli":
            return ["q", "chat", "--resume"]
        case "claude-code":
            // TODO: Implement claude-code resume command
            return nil
        default:
            return nil
        }
    }
}

extension AgentManager {
    
    /// Get agent command for a taskspace based on its state and selected agent
    func getAgentCommand(for taskspace: Taskspace, selectedAgent: String) -> [String]? {
        guard let agentInfo = availableAgents.first(where: { $0.id == selectedAgent }) else {
            return nil
        }
        
        switch taskspace.state {
        case .hatchling(let initialPrompt):
            return agentInfo.getHatchlingCommand(initialPrompt: initialPrompt)
            
        case .resume:
            return agentInfo.getResumeCommand()
        }
    }
}
