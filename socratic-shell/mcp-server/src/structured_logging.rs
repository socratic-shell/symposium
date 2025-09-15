//! Structured logging utilities for multi-component traceability
//!
//! Provides consistent log formatting across daemon, MCP server, and extension components.
//! Each log entry includes component type, process ID, and structured message.

use std::fmt;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::{FormatEvent, FormatFields, format::Writer};

use crate::constants;

/// Component types for logging identification
#[derive(Debug, Clone, Copy)]
pub enum Component {
    Daemon,
    McpServer,
    Client,
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Component::Daemon => write!(f, "DAEMON"),
            Component::McpServer => write!(f, "MCP-SERVER"),
            Component::Client => write!(f, "CLIENT"),
        }
    }
}

/// Custom formatter that adds component and PID prefixes to all log messages
pub struct ComponentFormatter {
    component: Component,
    pid: u32,
}

impl ComponentFormatter {
    pub fn new(component: Component) -> Self {
        Self {
            component,
            pid: std::process::id(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for ComponentFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Write the component prefix
        write!(&mut writer, "[{}:{}] ", self.component, self.pid)?;

        // Write the log level
        let level = *event.metadata().level();
        let level_str = match level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };
        write!(&mut writer, "{} ", level_str)?;

        // Write the message
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(&mut writer)?;
        Ok(())
    }
}

/// Initialize tracing with component-prefixed logging
pub fn init_component_tracing(
    component: Component,
    enable_dev_log: bool,
) -> Result<Option<tracing_appender::non_blocking::WorkerGuard>, Box<dyn std::error::Error>> {
    let formatter = ComponentFormatter::new(component);

    if enable_dev_log {
        use std::fs::OpenOptions;
        use tracing_appender::non_blocking;

        // Create file writer for dev logging
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(crate::constants::dev_log_path())?;

        let (file_writer, guard) = non_blocking(file);

        tracing_subscriber::fmt()
            .event_format(formatter)
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(file_writer)
            .init();

        eprintln!(
            "Development logging enabled - writing to {} (PID: {})",
            constants::dev_log_path(),
            std::process::id()
        );

        Ok(Some(guard))
    } else {
        tracing_subscriber::fmt()
            .event_format(formatter)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .init();

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_display() {
        assert_eq!(format!("{}", Component::Daemon), "DAEMON");
        assert_eq!(format!("{}", Component::McpServer), "MCP-SERVER");
        assert_eq!(format!("{}", Component::Client), "CLIENT");
    }
}
