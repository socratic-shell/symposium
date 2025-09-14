import AppKit
import Foundation

enum AgentType: String, CaseIterable, Identifiable {
    case qcli = "qcli"
    case claude = "claude"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .qcli: return "Amazon Q CLI"
        case .claude: return "Claude Code"
        }
    }
}

class AgentManager: ObservableObject {
    @Published var availableAgents: [AgentInfo] = []
    @Published var scanningCompleted = false
    @Published var scanningInProgress = false

    init() {
        Logger.shared.log("AgentManager: Created")
        scanForAgents(force: false)
    }

    func scanForAgents(force: Bool) {
        if !force {
            if self.scanningInProgress || self.scanningCompleted {
                return
            }
        }

        Logger.shared.log("AgentManager: Starting agent scan...")
        self.scanningInProgress = true

        DispatchQueue.global(qos: .userInitiated).async {
            var agents: [AgentInfo] = []

            // Check for Q CLI
            Logger.shared.log("AgentManager: Checking for Q CLI...")
            if let qcliInfo = self.detectQCLI() {
                agents.append(qcliInfo)
                Logger.shared.log("AgentManager: Q CLI detected: \(qcliInfo.statusText)")
            } else {
                Logger.shared.log("AgentManager: Q CLI not found")
            }

            // Check for Claude Code
            Logger.shared.log("AgentManager: Checking for Claude Code...")
            if let claudeInfo = self.detectClaudeCode() {
                agents.append(claudeInfo)
                Logger.shared.log("AgentManager: Claude Code detected: \(claudeInfo.statusText)")
            } else {
                Logger.shared.log("AgentManager: Claude Code not found")
            }

            DispatchQueue.main.async {
                self.availableAgents = agents
                self.scanningCompleted = true  // Always set to true when scanning completes
                self.scanningInProgress = false
                Logger.shared.log("AgentManager: Scan complete. Found \(agents.count) agents.")
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
            type: .qcli,
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
        Logger.shared.log("AgentManager: Looking for Claude Code executable...")

        // First try to find claude in PATH
        if let path = findExecutable(name: "claude") {
            Logger.shared.log("AgentManager: Found claude at: \(path)")
            let version = getClaudeCodeVersion(path: path)
            Logger.shared.log("AgentManager: Claude version: \(version ?? "unknown")")
            let (mcpConfigured, mcpPath) = checkClaudeCodeMCPConfiguration(claudePath: path)

            return AgentInfo(
                type: .claude,
                name: "Claude Code",
                description: "Anthropic Claude for coding",
                executablePath: path,
                version: version,
                isInstalled: true,
                isMCPConfigured: mcpConfigured,
                mcpServerPath: mcpPath
            )
        }

        Logger.shared.log("AgentManager: Claude not found in PATH, checking common locations...")

        // If not found in PATH, check common installation paths
        let possiblePaths = [
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "~/.local/bin/claude",
            "~/.volta/bin/claude",
        ].map { NSString(string: $0).expandingTildeInPath }

        for path in possiblePaths {
            Logger.shared.log("AgentManager: Checking: \(path)")
            if FileManager.default.isExecutableFile(atPath: path) {
                Logger.shared.log("AgentManager: Found executable at: \(path)")
                let version = getClaudeCodeVersion(path: path)
                let (mcpConfigured, mcpPath) = checkClaudeCodeMCPConfiguration(claudePath: path)

                return AgentInfo(
                    type: .claude,
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

        Logger.shared.log("AgentManager: Claude Code not found anywhere")

        // Return not installed info
        return AgentInfo(
            type: .claude,
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
                let output = String(data: data, encoding: .utf8)?.trimmingCharacters(
                    in: .whitespacesAndNewlines)
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
            let stdoutOutput = String(data: stdoutData, encoding: .utf8)?.trimmingCharacters(
                in: .whitespacesAndNewlines)

            if let stdout = stdoutOutput, !stdout.isEmpty {
                return stdout
            }

            // If stdout is empty, try stderr (Q CLI outputs to stderr)
            let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
            let stderrOutput = String(data: stderrData, encoding: .utf8)?.trimmingCharacters(
                in: .whitespacesAndNewlines)

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

        Logger.shared.log("AgentManager: Claude MCP command: \(claudePath) mcp list")
        Logger.shared.log("AgentManager: Claude MCP output: \(output ?? "nil")")

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
                return (false, nil)  // Found but not connected
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
    let type: AgentType
    let name: String
    let description: String
    let executablePath: String?
    let version: String?
    let isInstalled: Bool
    let isMCPConfigured: Bool
    let mcpServerPath: String?

    var id: String { type.id }

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

        switch type {
        case .qcli:
            return [
                "q", "chat",
                "To get your initialization instructions and project context, use the `expand_reference` tool with the argument 'yiasou'.",
            ]
        case .claude:
            // TODO: Implement claude-code hatchling command
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
    func getAgentCommand(for taskspace: Taskspace, selectedAgent: AgentType) -> [String]? {
        guard let agentInfo = availableAgents.first(where: { $0.type == selectedAgent }) else {
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
