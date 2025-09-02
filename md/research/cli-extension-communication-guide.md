# CLI Tool to VSCode Extension Communication Patterns

When you want a CLI command running in VSCode's terminal to communicate with your VSCode extension, there are several established patterns. Here are the most reliable cross-platform approaches:

## 1. Unix Socket/Named Pipe Pattern (Recommended)

This is the most secure and reliable approach used by many VSCode extensions including the built-in Git extension.

### Extension Side (IPC Server)

```typescript
import * as vscode from 'vscode';
import * as net from 'net';
import * as path from 'path';
import * as fs from 'fs';

export function activate(context: vscode.ExtensionContext) {
    const server = createIPCServer(context);
    
    // Pass the socket path to terminals via environment variable
    const socketPath = getSocketPath(context);
    context.environmentVariableCollection.replace("MY_EXTENSION_IPC_PATH", socketPath);
}

function createIPCServer(context: vscode.ExtensionContext): net.Server {
    const socketPath = getSocketPath(context);
    
    // Clean up any existing socket
    if (fs.existsSync(socketPath)) {
        fs.unlinkSync(socketPath);
    }
    
    const server = net.createServer((socket) => {
        console.log('CLI client connected');
        
        socket.on('data', (data) => {
            try {
                const message = JSON.parse(data.toString());
                handleCliMessage(message, socket);
            } catch (error) {
                console.error('Failed to parse CLI message:', error);
            }
        });
        
        socket.on('error', (error) => {
            console.error('Socket error:', error);
        });
    });
    
    server.listen(socketPath);
    
    // Clean up on extension deactivation
    context.subscriptions.push({
        dispose: () => {
            server.close();
            if (fs.existsSync(socketPath)) {
                fs.unlinkSync(socketPath);
            }
        }
    });
    
    return server;
}

function getSocketPath(context: vscode.ExtensionContext): string {
    // Use workspace-specific storage to avoid conflicts
    const storageUri = context.storageUri || context.globalStorageUri;
    const socketDir = storageUri.fsPath;
    
    // Ensure directory exists
    if (!fs.existsSync(socketDir)) {
        fs.mkdirSync(socketDir, { recursive: true });
    }
    
    // Platform-specific socket naming
    if (process.platform === 'win32') {
        return `\\\\.\\pipe\\my-extension-${Date.now()}`;
    } else {
        return path.join(socketDir, 'my-extension.sock');
    }
}

function handleCliMessage(message: any, socket: net.Socket) {
    switch (message.command) {
        case 'openFile':
            vscode.window.showTextDocument(vscode.Uri.file(message.path));
            socket.write(JSON.stringify({ status: 'success' }));
            break;
        case 'showMessage':
            vscode.window.showInformationMessage(message.text);
            socket.write(JSON.stringify({ status: 'success' }));
            break;
        default:
            socket.write(JSON.stringify({ 
                status: 'error', 
                message: 'Unknown command' 
            }));
    }
}
```

### CLI Tool Side

```bash
#!/bin/bash
# my-cli-tool.sh

# Check if we're running in VSCode terminal
if [ -z "$MY_EXTENSION_IPC_PATH" ]; then
    echo "Error: Not running in VSCode terminal with extension support"
    exit 1
fi

# Function to send message to VSCode extension
send_to_vscode() {
    local command="$1"
    local data="$2"
    
    local message="{\"command\":\"$command\",\"data\":$data}"
    
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        # Windows named pipe
        echo "$message" | nc -U "$MY_EXTENSION_IPC_PATH" 2>/dev/null
    else
        # Unix socket
        echo "$message" | socat - "UNIX-CONNECT:$MY_EXTENSION_IPC_PATH" 2>/dev/null
    fi
}

# Example usage
send_to_vscode "showMessage" "{\"text\":\"Hello from CLI!\"}"
send_to_vscode "openFile" "{\"path\":\"/path/to/file.txt\"}"
```

For Node.js-based CLI tools:

```javascript
#!/usr/bin/env node
const net = require('net');

function sendToVSCode(command, data) {
    return new Promise((resolve, reject) => {
        const socketPath = process.env.MY_EXTENSION_IPC_PATH;
        
        if (!socketPath) {
            reject(new Error('Not running in VSCode terminal with extension support'));
            return;
        }
        
        const client = net.createConnection(socketPath, () => {
            const message = JSON.stringify({ command, ...data });
            client.write(message);
        });
        
        client.on('data', (data) => {
            try {
                const response = JSON.parse(data.toString());
                resolve(response);
            } catch (error) {
                reject(error);
            }
            client.end();
        });
        
        client.on('error', reject);
    });
}

// Example usage
async function main() {
    try {
        await sendToVSCode('showMessage', { text: 'Hello from Node CLI!' });
        await sendToVSCode('openFile', { path: '/path/to/file.txt' });
    } catch (error) {
        console.error('Failed to communicate with VSCode:', error.message);
        process.exit(1);
    }
}

main();
```

## 2. HTTP Server Pattern

For simpler scenarios or when you need web-based communication:

### Extension Side

```typescript
import * as vscode from 'vscode';
import * as http from 'http';

export function activate(context: vscode.ExtensionContext) {
    const server = createHttpServer(context);
    const port = 0; // Let system assign port
    
    server.listen(port, 'localhost', () => {
        const address = server.address() as any;
        const actualPort = address.port;
        
        // Pass port to terminals
        context.environmentVariableCollection.replace("MY_EXTENSION_HTTP_PORT", actualPort.toString());
    });
}

function createHttpServer(context: vscode.ExtensionContext): http.Server {
    const server = http.createServer((req, res) => {
        // Enable CORS
        res.setHeader('Access-Control-Allow-Origin', '*');
        res.setHeader('Access-Control-Allow-Methods', 'POST, GET, OPTIONS');
        res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
        
        if (req.method === 'OPTIONS') {
            res.writeHead(200);
            res.end();
            return;
        }
        
        if (req.method === 'POST') {
            let body = '';
            req.on('data', chunk => body += chunk);
            req.on('end', () => {
                try {
                    const message = JSON.parse(body);
                    handleHttpMessage(message, res);
                } catch (error) {
                    res.writeHead(400, { 'Content-Type': 'application/json' });
                    res.end(JSON.stringify({ error: 'Invalid JSON' }));
                }
            });
        } else {
            res.writeHead(405);
            res.end('Method not allowed');
        }
    });
    
    context.subscriptions.push({
        dispose: () => server.close()
    });
    
    return server;
}

function handleHttpMessage(message: any, res: http.ServerResponse) {
    switch (message.command) {
        case 'openFile':
            vscode.window.showTextDocument(vscode.Uri.file(message.path));
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ status: 'success' }));
            break;
        default:
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Unknown command' }));
    }
}
```

### CLI Tool Side

```bash
#!/bin/bash
# Send HTTP request to VSCode extension

if [ -z "$MY_EXTENSION_HTTP_PORT" ]; then
    echo "Error: Extension HTTP port not available"
    exit 1
fi

curl -X POST "http://localhost:$MY_EXTENSION_HTTP_PORT" \
     -H "Content-Type: application/json" \
     -d '{"command":"openFile","path":"/path/to/file.txt"}'
```

## 3. File-Based Communication Pattern

For scenarios where real-time communication isn't critical:

### Extension Side

```typescript
import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';

export function activate(context: vscode.ExtensionContext) {
    const communicationDir = getCommunicationDir(context);
    
    // Set up environment variable for CLI tools
    context.environmentVariableCollection.replace("MY_EXTENSION_COMM_DIR", communicationDir);
    
    // Watch for incoming messages
    const watcher = fs.watch(communicationDir, (eventType, filename) => {
        if (filename && filename.endsWith('.json')) {
            handleFileMessage(path.join(communicationDir, filename));
        }
    });
    
    context.subscriptions.push({
        dispose: () => watcher.close()
    });
}

function getCommunicationDir(context: vscode.ExtensionContext): string {
    const storageUri = context.storageUri || context.globalStorageUri;
    const commDir = path.join(storageUri.fsPath, 'cli-comm');
    
    if (!fs.existsSync(commDir)) {
        fs.mkdirSync(commDir, { recursive: true });
    }
    
    return commDir;
}

function handleFileMessage(filePath: string) {
    try {
        const content = fs.readFileSync(filePath, 'utf8');
        const message = JSON.parse(content);
        
        // Process the message
        switch (message.command) {
            case 'openFile':
                vscode.window.showTextDocument(vscode.Uri.file(message.path));
                break;
            case 'showMessage':
                vscode.window.showInformationMessage(message.text);
                break;
        }
        
        // Clean up the message file
        fs.unlinkSync(filePath);
        
        // Write response file if needed
        if (message.responseFile) {
            fs.writeFileSync(message.responseFile, JSON.stringify({ status: 'success' }));
        }
    } catch (error) {
        console.error('Failed to process file message:', error);
    }
}
```

### CLI Tool Side

```bash
#!/bin/bash

if [ -z "$MY_EXTENSION_COMM_DIR" ]; then
    echo "Error: Communication directory not available"
    exit 1
fi

# Create unique message file
MESSAGE_FILE="$MY_EXTENSION_COMM_DIR/msg_$(date +%s%N).json"

# Send message
cat > "$MESSAGE_FILE" << EOF
{
    "command": "openFile",
    "path": "/path/to/file.txt",
    "timestamp": $(date +%s)
}
EOF

echo "Message sent to VSCode extension"
```

## 4. Remote Execution Considerations

For remote environments (SSH, containers, WSL), the socket/named pipe pattern still works best:

### SSH/Remote

The `environmentVariableCollection` automatically propagates to remote terminals, so your IPC setup works seamlessly. The socket files are created on the remote filesystem.

### WSL

VSCode handles WSL communication transparently. Your extension runs in the WSL context, so Unix sockets work normally.

### Containers

In dev containers, the socket path needs to be in a volume that's accessible to both the extension and terminal processes. Use the workspace storage path which is typically mounted.

## Best Practices

1. **Security**: Always validate messages from CLI tools. Don't execute arbitrary commands.

2. **Error Handling**: Implement robust error handling for connection failures, especially when VSCode restarts.

3. **Cleanup**: Always clean up sockets/files when the extension deactivates.

4. **Cross-Platform**: Use `environmentVariableCollection` for reliable environment variable propagation.

5. **Workspace Isolation**: Use workspace-specific storage paths to avoid conflicts between different projects.

The Unix socket/named pipe pattern is recommended for most use cases as it's secure, efficient, and handles VSCode's multi-window scenarios well.