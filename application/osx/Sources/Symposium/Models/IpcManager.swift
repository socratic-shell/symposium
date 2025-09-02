import Foundation
import Combine

// MARK: - IPC Message Types

/// Base IPC message structure for communication with VSCode extension via daemon
struct IPCMessage: Codable {
    let type: String
    let payload: JsonBlob
    let id: String
    let shellPid: Int?
    
    private enum CodingKeys: String, CodingKey {
        case type, payload, id
        case shellPid = "shell_pid"
    }
}

/// Request from VSCode extension to determine if agent should launch for a taskspace
struct GetTaskspaceStatePayload: Codable {
    let taskspaceUuid: String
}

/// Response to get_taskspace_state with agent launch decision
struct TaskspaceStateResponse: Codable {
    let agentCommand: [String]
    let shouldLaunch: Bool
}

/// Request from MCP tool to create a new taskspace
struct SpawnTaskspacePayload: Codable {
    let projectPath: String
    let taskspaceUuid: String  // UUID of parent taskspace requesting the spawn
    let name: String
    let taskDescription: String
    let initialPrompt: String
}

/// Response to spawn_taskspace with new taskspace UUID
struct SpawnTaskspaceResponse: Codable {
    let newTaskspaceUuid: String
}

/// Progress update from MCP tool for taskspace activity logs
struct LogProgressPayload: Codable {
    let projectPath: String
    let taskspaceUuid: String
    let message: String
    let category: String
}

/// Request from MCP tool for user attention (highlights taskspace, dock badge)
struct SignalUserPayload: Codable {
    let projectPath: String
    let taskspaceUuid: String
    let message: String
}

/// Generic response payload for all IPC requests
struct ResponsePayload: Codable {
    let success: Bool
    let error: String?
    let data: Data?
}

// MARK: - IPC Message Handling Protocol

/// Result of attempting to handle an IPC message
enum MessageHandlingResult<T: Codable> {
    case handled(T?)
    case notForMe
}

/// Protocol for objects that can handle IPC messages (typically one per active project)
protocol IpcMessageDelegate: AnyObject {
    func handleGetTaskspaceState(_ payload: GetTaskspaceStatePayload, messageId: String) async -> MessageHandlingResult<TaskspaceStateResponse>
    func handleSpawnTaskspace(_ payload: SpawnTaskspacePayload, messageId: String) async -> MessageHandlingResult<SpawnTaskspaceResponse>
    func handleLogProgress(_ payload: LogProgressPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse>
    func handleSignalUser(_ payload: SignalUserPayload, messageId: String) async -> MessageHandlingResult<EmptyResponse>
}

/// Empty response type for messages that don't return data
struct EmptyResponse: Codable {}

// MARK: - IPC Manager

class IpcManager: ObservableObject {
    @Published var isConnected = false
    @Published var error: String?
    
    private var clientProcess: Process?
    private var inputPipe: Pipe?
    private var delegates: [IpcMessageDelegate] = []
    
    // MARK: - Delegate Management
    
    func addDelegate(_ delegate: IpcMessageDelegate) {
        delegates.append(delegate)
        Logger.shared.log("IpcManager: Added delegate, now have \(delegates.count) delegates")
    }
    
    func removeDelegate(_ delegate: IpcMessageDelegate) {
        delegates.removeAll { $0 === delegate }
        Logger.shared.log("IpcManager: Removed delegate, now have \(delegates.count) delegates")
    }
    
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
        inputPipe = nil
        
        DispatchQueue.main.async {
            self.isConnected = false
        }
    }
    
    private func launchClient(mcpServerPath: String) {
        let process = Process()
        
        // Use shell to handle PATH resolution automatically
        process.executableURL = URL(fileURLWithPath: "/bin/sh")
        process.arguments = ["-c", "\(mcpServerPath) client"]
        
        // Set up pipes for stdin/stdout/stderr
        let inputPipe = Pipe()
        let outputPipe = Pipe()
        
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = outputPipe
        
        self.inputPipe = inputPipe
        
        do {
            try process.run()
            self.clientProcess = process
            
            DispatchQueue.main.async {
                self.isConnected = true
                Logger.shared.log("IpcManager: Client process started successfully")
                Logger.shared.log("IpcManager: isConnected set to \(self.isConnected)")
            }
            
            // Set up continuous message reading
            self.setupMessageReader(outputPipe: outputPipe)
            
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
    
    private func setupMessageReader(outputPipe: Pipe) {
        let fileHandle = outputPipe.fileHandleForReading
        
        DispatchQueue.global(qos: .background).async {
            var buffer = Data()
            
            while self.clientProcess != nil {
                let chunk = fileHandle.availableData
                if chunk.isEmpty {
                    break // Process ended
                }
                
                buffer.append(chunk)
                
                // Process complete lines (messages are newline-delimited)
                while let newlineRange = buffer.range(of: Data([0x0A])) { // \n
                    let lineData = buffer.subdata(in: 0..<newlineRange.lowerBound)
                    buffer.removeSubrange(0..<newlineRange.upperBound)
                    
                    if let lineString = String(data: lineData, encoding: .utf8), !lineString.isEmpty {
                        self.handleIncomingMessage(lineString)
                    }
                }
            }
        }
    }
    
    private func handleIncomingMessage(_ messageString: String) {
        Logger.shared.log("IpcManager: Received message: \(messageString)")
        
        guard let messageData = messageString.data(using: .utf8) else {
            Logger.shared.log("IpcManager: Failed to convert message to data")
            return
        }
        
        do {
            let message = try JSONDecoder().decode(IPCMessage.self, from: messageData)
            
            switch message.type {
            case "get_taskspace_state":
                handleGetTaskspaceState(message: message)
            case "spawn_taskspace":
                handleSpawnTaskspace(message: message)
            case "log_progress":
                handleLogProgress(message: message)
            case "signal_user":
                handleSignalUser(message: message)
            default:
                Logger.shared.log("IpcManager: Unknown message type: \(message.type)")
            }
            
        } catch {
            Logger.shared.log("IpcManager: Failed to parse message: \(error)")
        }
    }
    
    private func handleGetTaskspaceState(message: IPCMessage) {
        Task {
            do {
                let payloadData = try JSONEncoder().encode(message.payload)
                let payload = try JSONDecoder().decode(GetTaskspaceStatePayload.self, from: payloadData)
                Logger.shared.log("IpcManager: Get taskspace state for UUID: \(payload.taskspaceUuid)")
                
                // Try each delegate until one handles the message
                for delegate in delegates {
                    let result = await delegate.handleGetTaskspaceState(payload, messageId: message.id)
                    if case .handled(let responseData) = result {
                        sendResponse(to: message.id, success: true, data: responseData)
                        return
                    }
                }
                
                // No delegate handled the message
                Logger.shared.log("IpcManager: No delegate handled get_taskspace_state for UUID: \(payload.taskspaceUuid)")
                sendResponse(to: message.id, success: false, data: nil as EmptyResponse?, error: "Taskspace not found")
                
            } catch {
                Logger.shared.log("IpcManager: Failed to parse get_taskspace_state payload: \(error)")
                sendResponse(to: message.id, success: false, data: nil as TaskspaceStateResponse?, error: "Invalid payload")
            }
        }
    }
    
    private func handleSpawnTaskspace(message: IPCMessage) {
        Task {
            do {
                let payloadData = try JSONEncoder().encode(message.payload)
                let payload = try JSONDecoder().decode(SpawnTaskspacePayload.self, from: payloadData)
                Logger.shared.log("IpcManager: Spawn taskspace: \(payload.name) in \(payload.projectPath)")
                
                // Try each delegate until one handles the message
                for delegate in delegates {
                    let result = await delegate.handleSpawnTaskspace(payload, messageId: message.id)
                    if case .handled(let responseData) = result {
                        sendResponse(to: message.id, success: true, data: responseData)
                        return
                    }
                }
                
                // No delegate handled the message
                Logger.shared.log("IpcManager: No delegate handled spawn_taskspace for project: \(payload.projectPath)")
                sendResponse(to: message.id, success: false, data: nil as SpawnTaskspaceResponse?, error: "Project not found")
                
            } catch {
                Logger.shared.log("IpcManager: Failed to parse spawn_taskspace payload: \(error)")
                sendResponse(to: message.id, success: false, data: nil as SpawnTaskspaceResponse?, error: "Invalid payload")
            }
        }
    }
    
    private func handleLogProgress(message: IPCMessage) {
        Task {
            do {
                let payloadData = try JSONEncoder().encode(message.payload)
                let payload = try JSONDecoder().decode(LogProgressPayload.self, from: payloadData)
                Logger.shared.log("IpcManager: Log progress for \(payload.taskspaceUuid): \(payload.message)")
                
                // Try each delegate until one handles the message
                for delegate in delegates {
                    let result = await delegate.handleLogProgress(payload, messageId: message.id)
                    if case .handled(let responseData) = result {
                        sendResponse(to: message.id, success: true, data: responseData)
                        return
                    }
                }
                
                // No delegate handled the message
                Logger.shared.log("IpcManager: No delegate handled log_progress for UUID: \(payload.taskspaceUuid)")
                sendResponse(to: message.id, success: false, data: nil as EmptyResponse?, error: "Taskspace not found")
                
            } catch {
                Logger.shared.log("IpcManager: Failed to parse log_progress payload: \(error)")
                sendResponse(to: message.id, success: false, data: nil as EmptyResponse?, error: "Invalid payload")
            }
        }
    }
    
    private func handleSignalUser(message: IPCMessage) {
        Task {
            do {
                let payloadData = try JSONEncoder().encode(message.payload)
                let payload = try JSONDecoder().decode(SignalUserPayload.self, from: payloadData)
                Logger.shared.log("IpcManager: Signal user for \(payload.taskspaceUuid): \(payload.message)")
                
                // Try each delegate until one handles the message
                for delegate in delegates {
                    let result = await delegate.handleSignalUser(payload, messageId: message.id)
                    if case .handled(let responseData) = result {
                        sendResponse(to: message.id, success: true, data: responseData)
                        return
                    }
                }
                
                // No delegate handled the message
                Logger.shared.log("IpcManager: No delegate handled signal_user for UUID: \(payload.taskspaceUuid)")
                sendResponse(to: message.id, success: false, data: nil as EmptyResponse?, error: "Taskspace not found")
                
            } catch {
                Logger.shared.log("IpcManager: Failed to parse signal_user payload: \(error)")
                sendResponse(to: message.id, success: false, data: nil as EmptyResponse?, error: "Invalid payload")
            }
        }
    }
    
    // MARK: - Response Sending (for delegates)
    
    func sendResponse<T: Codable>(to messageId: String, success: Bool, data: T? = nil, error: String? = nil) {
        guard let inputPipe = self.inputPipe else {
            Logger.shared.log("IpcManager: Cannot send response - no input pipe")
            return
        }
        
        do {
            let responseData: Data?
            if let data = data {
                responseData = try JSONEncoder().encode(data)
            } else {
                responseData = nil
            }
            
            let responsePayload = ResponsePayload(success: success, error: error, data: responseData)
            let encodedResponseData = try JSONEncoder().encode(responsePayload)
            let responseMessage = IPCMessage(
                type: "response",
                payload: try JSONDecoder().decode(JsonBlob.self, from: encodedResponseData),
                id: messageId,
                shellPid: nil
            )
            
            let messageData = try JSONEncoder().encode(responseMessage)
            var messageString = String(data: messageData, encoding: .utf8) ?? ""
            messageString += "\n"
            
            if let stringData = messageString.data(using: String.Encoding.utf8) {
                inputPipe.fileHandleForWriting.write(stringData)
                Logger.shared.log("IpcManager: Sent response to \(messageId)")
            }
            
        } catch {
            Logger.shared.log("IpcManager: Failed to send response: \(error)")
        }
    }
}
