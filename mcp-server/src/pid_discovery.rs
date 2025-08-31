//! PID Discovery for VSCode Integration
//!
//! Walks up the process tree to find VSCode PID and terminal shell PID
//! for reliable IPC connection establishment.

use anyhow::Result;
use std::process::Command;
use tracing::{debug, error, info};

/// Walk up the process tree to find VSCode PID and terminal shell PID
pub async fn find_vscode_pid_from_mcp(start_pid: u32) -> Result<Option<(u32, u32)>> {
    let mut current_pid = start_pid;
    let mut process_chain = Vec::new();

    info!("Starting PID walk from MCP server PID: {}", start_pid);

    // Walk up the process tree (safety limit of 10 levels)
    for _i in 0..10 {
        // Get process information using ps command
        let output = Command::new("ps")
            .args(&["-p", &current_pid.to_string(), "-o", "pid,ppid,comm,args"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.trim().split('\n').collect();

                if lines.len() < 2 {
                    error!("No process info found for PID {}", current_pid);
                    break;
                }

                let process_line = lines[1].trim();
                let parts: Vec<&str> = process_line.split_whitespace().collect();

                if parts.len() < 4 {
                    error!("Malformed process line: {}", process_line);
                    break;
                }

                let pid: u32 = parts[0].parse().unwrap_or(0);
                let ppid: u32 = parts[1].parse().unwrap_or(0);
                let command = parts[3..].join(" ");

                debug!("  PID {pid} -> PPID {ppid}: {command}");

                // Store this process in our chain
                process_chain.push((pid, ppid, command.clone()));

                // Check if this looks like the main VSCode process (not helper processes)
                if (command.contains("Visual Studio Code")
                    || command.contains("Code.app")
                    || command.contains("Electron"))
                    && !command.contains("Code Helper")
                {
                    info!("Found main VSCode process: pid {pid}");

                    // Find the terminal shell PID by looking for the immediate child of VSCode
                    // that looks like a terminal (contains shell names or "(qterm)")
                    let terminal_shell_pid = find_terminal_shell_in_chain(&process_chain, pid);

                    if let Some(shell_pid) = terminal_shell_pid {
                        return Ok(Some((pid, shell_pid)));
                    } else {
                        info!("  Warning: Found VSCode but could not identify terminal shell PID");
                        return Ok(Some((pid, current_pid))); // Fallback
                    }
                }

                current_pid = ppid;
                if ppid <= 1 {
                    info!("Reached init process (PID 1)");
                    break;
                }
            }
            Ok(output) => {
                error!("ps command failed with status: {}", output.status);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    error!("ps stderr: {}", stderr);
                }
                break;
            }
            Err(e) => {
                error!("Failed to execute ps command: {}", e);
                break;
            }
        }
    }

    error!("Reached end of process tree without finding VSCode");
    Ok(None)
}

/// Find the terminal shell PID in the process chain
/// Looks for the immediate child of VSCode that appears to be a terminal shell
fn find_terminal_shell_in_chain(
    process_chain: &[(u32, u32, String)],
    _vscode_pid: u32,
) -> Option<u32> {
    // First priority: Look for processes with (qterm) which clearly indicates a terminal
    for (pid, _ppid, command) in process_chain.iter().rev() {
        if command.contains("(qterm)") {
            info!("  Found terminal shell (qterm): PID {} ({})", pid, command);
            return Some(*pid);
        }
    }

    // Second priority: Look for shell processes that are not login shells or command runners
    for (pid, _ppid, command) in process_chain.iter().rev() {
        if (command.contains("zsh") && !command.contains("--login") && !command.contains("-c"))
            || (command.contains("bash") && !command.contains("-c") && !command.contains("--login"))
            || command.contains("fish")
        {
            info!("  Found terminal shell (shell): PID {} ({})", pid, command);
            return Some(*pid);
        }
    }

    // Third priority: Any shell-like process as fallback
    for (pid, _ppid, command) in process_chain.iter().rev() {
        if command.contains("zsh") || command.contains("bash") || command.contains("sh") {
            info!("  Found fallback shell: PID {} ({})", pid, command);
            return Some(*pid);
        }
    }

    info!("  No terminal shell found in process chain");
    None
}
