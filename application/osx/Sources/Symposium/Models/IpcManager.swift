import Foundation
import Combine

// MARK: - IPC Message Types

struct IPCMessage: Codable {
    let type: String
    let payload: Data
    let id: String
    let shellPid: Int?
    
    private enum CodingKeys: String, CodingKey {
        case type, payload, id
        case shellPid = "shell_pid"
    }
}

struct GetTaskspaceStatePayload: Codable {
    let taskspaceUuid: String
}

struct TaskspaceStateResponse: Codable {
    let agentCommand: [String]
    let shouldLaunch: Bool
}

struct SpawnTaskspacePayload: Codable {
    let projectPath: String
    let taskspaceUuid: String
    let name: String
    let taskDescription: String
    let initialPrompt: String
}

struct LogProgressPayload: Codable {
    let projectPath: String
    let taskspaceUuid: String
    let message: String
    let category: String
}

struct SignalUserPayload: Codable {
    let projectPath: String
    let taskspaceUuid: String
    let message: String
}

struct ResponsePayload: Codable {
    let success: Bool
    let error: String?
    let data: Data?
}

// MARK: - IPC Manager

class IpcManager: ObservableObject {
    @Published var isConnected = false
    @Published var error: String?
    
    private var clientProcess: Process?
    private var inputPipe: Pipe?
    
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
        process.launchPath = mcpServerPath
        process.arguments = ["client"]
        
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
        do {
            let payload = try JSONDecoder().decode(GetTaskspaceStatePayload.self, from: message.payload)
            Logger.shared.log("IpcManager: Get taskspace state for UUID: \(payload.taskspaceUuid)")
            
            // TODO: Look up taskspace by UUID and determine agent command
            let response = TaskspaceStateResponse(
                agentCommand: ["q", "chat"], // TODO: Get from user preferences and taskspace state
                shouldLaunch: true // TODO: Check if taskspace exists and is not complete
            )
            
            sendResponse(to: message.id, success: true, data: response)
            
        } catch {
            Logger.shared.log("IpcManager: Failed to parse get_taskspace_state payload: \(error)")
            sendResponse(to: message.id, success: false, error: "Invalid payload")
        }
    }
    
    private func handleSpawnTaskspace(message: IPCMessage) {
        do {
            let payload = try JSONDecoder().decode(SpawnTaskspacePayload.self, from: message.payload)
            Logger.shared.log("IpcManager: Spawn taskspace: \(payload.name) in \(payload.projectPath)")
            
            // TODO: Create taskspace directory, clone repo, save metadata
            
            sendResponse(to: message.id, success: true, data: nil)
            
        } catch {
            Logger.shared.log("IpcManager: Failed to parse spawn_taskspace payload: \(error)")
            sendResponse(to: message.id, success: false, error: "Invalid payload")
        }
    }
    
    private func handleLogProgress(message: IPCMessage) {
        do {
            let payload = try JSONDecoder().decode(LogProgressPayload.self, from: message.payload)
            Logger.shared.log("IpcManager: Log progress for \(payload.taskspaceUuid): \(payload.message)")
            
            // TODO: Update taskspace logs and save to taskspace.json
            
            sendResponse(to: message.id, success: true, data: nil)
            
        } catch {
            Logger.shared.log("IpcManager: Failed to parse log_progress payload: \(error)")
            sendResponse(to: message.id, success: false, error: "Invalid payload")
        }
    }
    
    private func handleSignalUser(message: IPCMessage) {
        do {
            let payload = try JSONDecoder().decode(SignalUserPayload.self, from: message.payload)
            Logger.shared.log("IpcManager: Signal user for \(payload.taskspaceUuid): \(payload.message)")
            
            // TODO: Update taskspace attention flag and dock badge
            
            sendResponse(to: message.id, success: true, data: nil)
            
        } catch {
            Logger.shared.log("IpcManager: Failed to parse signal_user payload: \(error)")
            sendResponse(to: message.id, success: false, error: "Invalid payload")
        }
    }
    
    private func sendResponse<T: Codable>(to messageId: String, success: Bool, data: T? = nil, error: String? = nil) {
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
            let responseMessage = IPCMessage(
                type: "response",
                payload: try JSONEncoder().encode(responsePayload),
                id: messageId,
                shellPid: nil
            )
            
            let messageData = try JSONEncoder().encode(responseMessage)
            var messageString = String(data: messageData, encoding: .utf8) ?? ""
            messageString += "\n"
            
            if let stringData = messageString.data(using: .utf8) {
                inputPipe.fileHandleForWriting.write(stringData)
                Logger.shared.log("IpcManager: Sent response to \(messageId)")
            }
            
        } catch {
            Logger.shared.log("IpcManager: Failed to send response: \(error)")
        }
    }
}
