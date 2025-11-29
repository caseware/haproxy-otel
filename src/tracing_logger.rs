use tracing::{Event, Subscriber};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::sync::Mutex;
use std::collections::VecDeque;

/// A global queue for log messages that will be written to stderr
/// since HAProxy Core.log requires a Lua context which isn't available in background threads
static LOG_BUFFER: Mutex<Option<VecDeque<String>>> = Mutex::new(None);

/// Initialize the log buffer
fn init_log_buffer() {
    let mut buffer = LOG_BUFFER.lock().unwrap();
    if buffer.is_none() {
        *buffer = Some(VecDeque::new());
    }
}

/// Log a message - writes directly to stderr since we can't access HAProxy Core from background threads
fn log_message(message: String) {
    // Write to stderr which HAProxy will capture in its logs
    eprintln!("{}", message);
}

/// A custom tracing layer that forwards all OpenTelemetry internal logs to HAProxy's logging system
pub(crate) struct HaproxyTracingLayer {
    log_level: Option<String>,
}

impl HaproxyTracingLayer {
    pub fn new(log_level: Option<String>) -> Self {
        Self { log_level }
    }

    fn should_log(&self) -> bool {
        matches!(
            self.log_level.as_deref(),
            Some("debug") | Some("info") | Some("warning") | Some("error")
        )
    }

    fn should_log_level(&self, level: &tracing::Level) -> bool {
        if !self.should_log() {
            return false;
        }

        match self.log_level.as_deref() {
            Some("debug") => true, // Log everything
            Some("info") => matches!(*level, tracing::Level::ERROR | tracing::Level::WARN | tracing::Level::INFO),
            Some("warning") => matches!(*level, tracing::Level::ERROR | tracing::Level::WARN),
            Some("error") => matches!(*level, tracing::Level::ERROR),
            _ => false,
        }
    }

    fn format_log(&self, level: &tracing::Level, message: &str) -> String {
        let level_str = match *level {
            tracing::Level::ERROR => "ERROR",
            tracing::Level::WARN => "WARN",
            tracing::Level::INFO => "INFO",
            tracing::Level::DEBUG => "DEBUG",
            tracing::Level::TRACE => "TRACE",
        };
        format!("[haproxy-otel] [{}] {}", level_str, message)
    }
}

impl<S> Layer<S> for HaproxyTracingLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if !self.should_log_level(event.metadata().level()) {
            return;
        }

        let metadata = event.metadata();
        
        // Format the event message
        let mut message = String::new();
        
        // Add target (module path) for context
        let target = metadata.target();
        if target.contains("opentelemetry") || target.contains("otlp") || target.contains("reqwest") || target.contains("hyper") {
            message.push_str(&format!("[{}] ", target));
        }

        // Visit the event fields to extract the message
        let mut visitor = MessageVisitor { message: &mut message };
        event.record(&mut visitor);

        if !message.is_empty() {
            let formatted = self.format_log(metadata.level(), &message);
            log_message(formatted);
        }
    }

    fn on_new_span(&self, _attrs: &Attributes<'_>, _id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // We don't need to log span creation, just events
    }

    fn on_record(&self, _id: &Id, _values: &Record<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // We don't need to log span updates
    }

    fn on_enter(&self, _id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // We don't need to log span entry
    }

    fn on_exit(&self, _id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // We don't need to log span exit
    }

    fn on_close(&self, _id: Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // We don't need to log span close
    }
}

/// A visitor to extract the message from tracing events
struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message.push_str(&format!("{:?}", value));
        } else {
            if !self.message.is_empty() {
                self.message.push_str(", ");
            }
            self.message.push_str(&format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            if !self.message.is_empty() {
                self.message.push_str(", ");
            }
            self.message.push_str(&format!("{}={}", field.name(), value));
        }
    }
}

/// Initialize the tracing subscriber with HAProxy logging
pub(crate) fn init_tracing(log_level: Option<String>) {
    // Check if already initialized
    if tracing::dispatcher::has_been_set() {
        return;
    }

    init_log_buffer();

    // Determine the tracing level based on configuration
    let filter_level = match log_level.as_deref() {
        Some("debug") => "debug",
        Some("info") => "info",
        Some("warning") | Some("warn") => "warn",
        Some("error") => "error",
        _ => return, // Don't initialize tracing if no log level is set
    };

    // Create an EnvFilter that enables debug logging for OpenTelemetry crates
    let env_filter = EnvFilter::new(filter_level)
        .add_directive("opentelemetry=debug".parse().unwrap())
        .add_directive("opentelemetry_sdk=debug".parse().unwrap())
        .add_directive("opentelemetry_otlp=debug".parse().unwrap())
        .add_directive("opentelemetry_http=debug".parse().unwrap())
        .add_directive("reqwest=debug".parse().unwrap())
        .add_directive("hyper=info".parse().unwrap()); // Hyper is too verbose at debug

    let haproxy_layer = HaproxyTracingLayer::new(log_level);

    // Create a subscriber with our custom layer and env filter
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(haproxy_layer);

    // Set as the global default subscriber
    let _ = subscriber.try_init();
}
