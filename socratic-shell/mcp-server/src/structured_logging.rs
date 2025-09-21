//! Structured logging utilities for multi-component traceability
//!
//! Provides consistent log formatting across daemon, MCP server, and extension components.
//! Each log entry includes component type, process ID, and structured message.

use std::fmt;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::{FormatEvent, FormatFields, format::Writer};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use crate::constants;
use crate::types::LogLevel;

/// Global log senders for subscriber communication
static LOG_SUBSCRIBERS: Mutex<Vec<mpsc::UnboundedSender<(LogLevel, String)>>> = Mutex::new(Vec::new());

/// Add a log subscriber and return the receiver
pub fn add_log_subscriber() -> mpsc::UnboundedReceiver<(LogLevel, String)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut subscribers = LOG_SUBSCRIBERS.lock().unwrap();
    subscribers.push(tx);
    rx
}

/// Send a log message to all subscribers
fn send_to_subscribers(level: LogLevel, message: String) {
    if let Ok(mut subscribers) = LOG_SUBSCRIBERS.lock() {
        // Send to all subscribers, removing any that are closed
        subscribers.retain(|sender| sender.send((level.clone(), message.clone())).is_ok());
    }
}

/// Custom tracing layer that sends logs to subscribers
pub struct DaemonLogLayer {
    component: Component,
    pid: u32,
}

impl DaemonLogLayer {
    pub fn new(component: Component) -> Self {
        Self {
            component,
            pid: std::process::id(),
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for DaemonLogLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Convert tracing level to our LogLevel
        let level = match *event.metadata().level() {
            Level::ERROR => LogLevel::Error,
            Level::WARN => LogLevel::Error, // Map WARN to Error for simplicity
            Level::INFO => LogLevel::Info,
            Level::DEBUG => LogLevel::Debug,
            Level::TRACE => LogLevel::Debug, // Map TRACE to Debug
        };

        // Format the message with component prefix
        let mut message = format!("[{}:{}] ", self.component, self.pid);
        
        // Extract the message from the event
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);
        message.push_str(&visitor.message);

        // Send to subscribers
        send_to_subscribers(level, message);
    }
}

/// Visitor to extract message from tracing event
struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove quotes from debug formatting
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len()-1].to_string();
            }
        }
    }
}

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

/// Initialize tracing with component-prefixed logging that sends to both stderr and daemon
pub fn init_component_tracing(
    component: Component,
    enable_dev_log: bool,
) -> Result<Option<tracing_appender::non_blocking::WorkerGuard>, Box<dyn std::error::Error>> {
    let formatter = ComponentFormatter::new(component);
    let daemon_layer = DaemonLogLayer::new(component);

    if enable_dev_log {
        use std::fs::OpenOptions;
        use tracing_appender::non_blocking;

        // Create file writer for dev logging
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(crate::constants::dev_log_path())?;

        let (file_writer, guard) = non_blocking(file);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .with_writer(file_writer)
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG)
            )
            .with(daemon_layer)
            .init();

        eprintln!(
            "Development logging enabled - writing to {} (PID: {})",
            constants::dev_log_path(),
            std::process::id()
        );

        Ok(Some(guard))
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .event_format(formatter)
                    .with_writer(std::io::stderr)
                    .with_filter(tracing_subscriber::EnvFilter::from_default_env())
            )
            .with(daemon_layer)
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
