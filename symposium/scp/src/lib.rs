//! Symposium Component Protocol (SCP)
//!
//! SCP extends ACP to enable composable agent architectures through proxy chains.
//! Each proxy in the chain can intercept and transform messages, adding capabilities
//! like walkthroughs, collaboration patterns, and IDE integrations.

use agent_client_protocol_schema::McpServer;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Request to initialize a proxy chain.
///
/// Sent as an extension method `_scp/proxy` from the editor to the first proxy.
/// The proxy then launches the downstream proxies/agent specified in this request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScpProxyRequest {
    /// The chain of proxies and final agent to initialize.
    /// Uses McpServer (reusing ACP's server configuration format).
    /// Initially only Stdio transport is supported.
    pub servers: Vec<McpServer>,
}

#[derive(Debug, Error)]
pub enum ScpError {
    #[error("Only stdio transport is supported for SCP proxies, got: {0}")]
    UnsupportedTransport(String),

    #[error("Proxy chain cannot be empty")]
    EmptyProxyChain,

    #[error("Failed to spawn proxy process: {0}")]
    ProcessSpawnError(#[from] std::io::Error),

    #[error("ACP protocol error: {0}")]
    AcpError(String),
}

/// Validates that an McpServer is using stdio transport.
/// Returns an error for Http or Sse variants.
pub fn validate_stdio_transport(server: &McpServer) -> Result<(), ScpError> {
    match server {
        McpServer::Stdio { .. } => Ok(()),
        McpServer::Http { name, .. } => Err(ScpError::UnsupportedTransport(format!(
            "http server '{}'",
            name
        ))),
        McpServer::Sse { name, .. } => Err(ScpError::UnsupportedTransport(format!(
            "sse server '{}'",
            name
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_stdio_transport() {
        let stdio_server = McpServer::Stdio {
            name: "test-proxy".to_string(),
            command: PathBuf::from("/usr/bin/proxy"),
            args: vec![],
            env: vec![],
        };

        assert!(validate_stdio_transport(&stdio_server).is_ok());
    }

    #[test]
    fn test_reject_http_transport() {
        let http_server = McpServer::Http {
            name: "test-http".to_string(),
            url: "http://example.com".to_string(),
            headers: vec![],
        };

        let result = validate_stdio_transport(&http_server);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http"));
    }
}
