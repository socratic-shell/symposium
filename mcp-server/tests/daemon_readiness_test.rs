//! Test for daemon readiness message functionality

use std::io::{BufRead, BufReader};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[tokio::test]
async fn test_daemon_readiness_message() {
    // Initialize tracing for test output
    let _ = tracing_subscriber::fmt::try_init();

    let test_pid = std::process::id(); // Use current process PID

    // Spawn daemon as separate process with stdout captured
    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "run",
        "-p",
        "symposium-mcp",
        "--",
        "daemon",
        "--prefix",
        "symposium-mcp-test_daemon_readiness_message",
        &test_pid.to_string(),
    ]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = ProcessCleanup {
        child: cmd.spawn().expect("Failed to spawn daemon process"),
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
    }
}
