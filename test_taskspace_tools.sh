#!/bin/bash

# Manual test script for taskspace MCP tools
# This script tests the new tools by calling them via the MCP protocol

set -e

echo "Testing taskspace MCP tools..."

# Set test mode to avoid actual IPC communication
export DIALECTIC_TEST_MODE=1

# Build the MCP server
echo "Building MCP server..."
cd mcp-server
cargo build --release
cd ..

# Test spawn_taskspace tool
echo "Testing spawn_taskspace tool..."
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "spawn_taskspace",
    "arguments": {
      "name": "test-taskspace",
      "task_description": "Test taskspace for verification",
      "initial_prompt": "Hello, this is a test prompt for the new taskspace"
    }
  }
}' | ./mcp-server/target/release/symposium-mcp server 2>/dev/null | jq .

# Test log_progress tool
echo "Testing log_progress tool..."
echo '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "log_progress",
    "arguments": {
      "message": "Test progress message",
      "category": "milestone"
    }
  }
}' | ./mcp-server/target/release/symposium-mcp server 2>/dev/null | jq .

# Test signal_user tool
echo "Testing signal_user tool..."
echo '{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "signal_user",
    "arguments": {
      "message": "Need assistance with this task"
    }
  }
}' | ./mcp-server/target/release/symposium-mcp server 2>/dev/null | jq .

echo "All tests completed successfully!"
