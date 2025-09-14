//! Test for daemon readiness message functionality

use std::io::{BufRead, BufReader};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[tokio::test]
async fn test_daemon_readiness_message() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    // Use a unique test prefix based on the test name
    let test_prefix = "socratic-shell-mcp-test_daemon_readiness_message";
    let socket_path = format!("/tmp/{}.sock", test_prefix);
    
    // Clean up any existing socket file before starting
    let _ = std::fs::remove_file(&socket_path);

    // Spawn daemon as separate process with stdout captured
    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "run",
        "-p",
        "socratic-shell-mcp",
        "--",
        "daemon",
        "--prefix",
        test_prefix,
    ]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = ProcessCleanup {
        child: cmd.spawn().expect("Failed to spawn daemon process"),
        socket_path,
    };

    println!("Spawned daemon process with PID: {}", child.id());

    // Read stdout until we get the DAEMON_READY message
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        // Use a timeout as a safety net
        let timeout_result = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::task::spawn_blocking(move || {
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            println!("Daemon output: {}", line);
                            if line.trim() == "DAEMON_READY" {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            return Err(format!("Error reading daemon stdout: {}", e));
                        }
                    }
                }
                Err("Daemon process ended without sending DAEMON_READY message".to_string())
            })
            .await
            .unwrap()
        })
        .await;

        match timeout_result {
            Ok(Ok(())) => {
                println!("✅ Successfully received DAEMON_READY message");
            }
            Ok(Err(e)) => {
                panic!("❌ Error reading daemon output: {}", e);
            }
            Err(_) => {
                panic!("❌ Timeout waiting for DAEMON_READY message");
            }
        }
    } else {
        panic!("❌ Failed to capture daemon stdout");
    }
}

struct ProcessCleanup {
    child: Child,
    socket_path: String,
}

impl Deref for ProcessCleanup {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl DerefMut for ProcessCleanup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl Drop for ProcessCleanup {
    fn drop(&mut self) {
        println!("Killing process with PID: {}", self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        
        // Clean up socket file
        if std::path::Path::new(&self.socket_path).exists() {
            if let Err(e) = std::fs::remove_file(&self.socket_path) {
                println!("Warning: Failed to clean up socket file {}: {}", self.socket_path, e);
            } else {
                println!("Cleaned up socket file: {}", self.socket_path);
            }
        }
    }
}
