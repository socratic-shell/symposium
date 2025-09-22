//! Structured logging utilities for multi-component traceability
//!
//! Provides consistent log formatting across daemon, MCP server, and extension components.
//! Each log entry includes component type, process ID, and structured message.

use std::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};
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
pub struct ForwardToSubscriberLayer;

impl<S> tracing_subscriber::Layer<S> for ForwardToSubscriberLayer
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
        
        // Extract the message from the event
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        // Send to subscribers
        send_to_subscribers(level, visitor.message);
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

/// Initialize tracing with component-prefixed logging that sends to both stderr and daemon
pub fn init_component_tracing(
    enable_dev_log: bool,
) -> Result<Option<tracing_appender::non_blocking::WorkerGuard>, Box<dyn std::error::Error>> {
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
                    .with_writer(file_writer)
                    .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG)
            )
            .with(ForwardToSubscriberLayer)
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
                    .with_writer(std::io::stderr)
                    .with_filter(tracing_subscriber::EnvFilter::from_default_env())
            )
            .with(ForwardToSubscriberLayer)
            .init();

        Ok(None)
    }
}
