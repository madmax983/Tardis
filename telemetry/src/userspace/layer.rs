//! Custom tracing layer for Tardis telemetry.
//!
//! This module provides `TardisLayer`, a `tracing::Layer` implementation
//! that captures spans and events for storage in Gallifrey and export
//! via OpenTelemetry.

use crate::types::{Level, SpanId, Subsystem, TraceId};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[cfg(feature = "std")]
use crate::gallifrey::TelemetryStore;

/// Data associated with an active span.
#[derive(Debug, Clone)]
pub struct SpanData {
    /// Trace ID for distributed tracing.
    pub trace_id: TraceId,
    /// Span ID.
    pub span_id: SpanId,
    /// Parent span ID (if any).
    pub parent_id: Option<SpanId>,
    /// Span name.
    pub name: String,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// When the span started (monotonic).
    pub start_instant: Instant,
    /// When the span started (wall clock).
    pub start_time: DateTime<Utc>,
    /// When the span ended (wall clock).
    pub end_time: Option<DateTime<Utc>>,
    /// Span attributes.
    pub attributes: HashMap<String, String>,
    /// Span level.
    pub level: Level,
}

impl SpanData {
    /// Returns the duration of the span in nanoseconds.
    #[must_use]
    pub fn duration_ns(&self) -> Option<u64> {
        self.end_time
            .map(|_| self.start_instant.elapsed().as_nanos() as u64)
    }
}

/// Data associated with an event.
#[derive(Debug, Clone)]
pub struct EventData {
    /// Span ID this event belongs to.
    pub span_id: Option<SpanId>,
    /// Trace ID.
    pub trace_id: TraceId,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event level.
    pub level: Level,
    /// Event message.
    pub message: String,
    /// Event fields.
    pub fields: HashMap<String, String>,
    /// Originating subsystem.
    pub subsystem: Subsystem,
}

/// Configuration for the Tardis telemetry layer.
#[derive(Debug, Clone)]
pub struct TardisLayerConfig {
    /// Enable storage to Gallifrey.
    pub gallifrey_enabled: bool,
    /// Enable OTLP export.
    pub otlp_enabled: bool,
    /// Minimum level to record.
    pub min_level: Level,
}

impl Default for TardisLayerConfig {
    fn default() -> Self {
        Self {
            gallifrey_enabled: true,
            otlp_enabled: false,
            min_level: Level::Info,
        }
    }
}

/// The main Tardis tracing layer.
///
/// This layer captures span lifecycle events and tracing events,
/// storing them to Gallifrey and/or exporting via OpenTelemetry.
pub struct TardisLayer {
    config: TardisLayerConfig,
    #[cfg(feature = "std")]
    store: Option<Arc<TelemetryStore>>,
    otlp_sender: Option<tokio::sync::mpsc::Sender<SpanData>>,
}

impl TardisLayer {
    /// Creates a new Tardis layer with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TardisLayerConfig::default(),
            #[cfg(feature = "std")]
            store: None,
            otlp_sender: None,
        }
    }

    /// Creates a new Tardis layer with the given configuration.
    #[must_use]
    pub fn with_config(config: TardisLayerConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "std")]
            store: None,
            otlp_sender: None,
        }
    }

    /// Sets the Gallifrey telemetry store.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn with_store(mut self, store: Arc<TelemetryStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Sets the OTLP sender for span export.
    #[must_use]
    pub fn with_otlp_sender(mut self, sender: tokio::sync::mpsc::Sender<SpanData>) -> Self {
        self.otlp_sender = Some(sender);
        self
    }

    /// Gets or creates a trace ID from the current context.
    fn get_or_create_trace_id<S>(&self, ctx: &Context<'_, S>) -> TraceId
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        // Walk up the span tree to find an existing trace ID
        if let Some(span) = ctx.lookup_current() {
            let mut current = Some(span);
            while let Some(s) = current {
                if let Some(data) = s.extensions().get::<SpanData>() {
                    return data.trace_id;
                }
                current = s.parent();
            }
        }

        // No existing trace, create a new one
        TraceId::generate()
    }

    /// Gets the parent span ID from the current context.
    fn get_parent_span_id<S>(&self, ctx: &Context<'_, S>) -> Option<SpanId>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        ctx.lookup_current()
            .and_then(|span| span.extensions().get::<SpanData>().map(|data| data.span_id))
    }
}

impl Default for TardisLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TardisLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TardisLayer")
            .field("config", &self.config)
            .finish()
    }
}

impl<S> Layer<S> for TardisLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let metadata = attrs.metadata();

        // Check level filter
        let level: Level = (*metadata.level()).into();
        if level < self.config.min_level {
            return;
        }

        let trace_id = self.get_or_create_trace_id(&ctx);
        let parent_id = self.get_parent_span_id(&ctx);
        let span_id = SpanId::generate();

        // Collect attributes
        let mut attributes = HashMap::new();
        let mut visitor = FieldVisitor(&mut attributes);
        attrs.record(&mut visitor);

        let span_data = SpanData {
            trace_id,
            span_id,
            parent_id,
            name: metadata.name().to_string(),
            subsystem: Subsystem::from_target(metadata.target()),
            start_instant: Instant::now(),
            start_time: Utc::now(),
            end_time: None,
            attributes,
            level,
        };

        span.extensions_mut().insert(span_data);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(span_data) = extensions.get_mut::<SpanData>() {
            let mut visitor = FieldVisitor(&mut span_data.attributes);
            values.record(&mut visitor);
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level: Level = (*metadata.level()).into();

        if level < self.config.min_level {
            return;
        }

        // Get trace context from current span
        let (trace_id, span_id) = if let Some(span) = ctx.lookup_current() {
            span.extensions()
                .get::<SpanData>()
                .map(|data| (data.trace_id, Some(data.span_id)))
                .unwrap_or((TraceId::NONE, None))
        } else {
            (TraceId::NONE, None)
        };

        // Collect event fields
        let mut fields = HashMap::new();
        let mut message = String::new();
        let mut visitor = EventVisitor {
            fields: &mut fields,
            message: &mut message,
        };
        event.record(&mut visitor);

        let event_data = EventData {
            span_id,
            trace_id,
            timestamp: Utc::now(),
            level,
            message,
            fields,
            subsystem: Subsystem::from_target(metadata.target()),
        };

        // Store to Gallifrey if enabled
        #[cfg(feature = "std")]
        if self.config.gallifrey_enabled {
            if let Some(store) = &self.store {
                let store = Arc::clone(store);
                let event_data = event_data.clone();
                tokio::spawn(async move {
                    let _ = store.record_event(event_data).await;
                });
            }
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("span not found");
        let mut extensions = span.extensions_mut();

        if let Some(mut span_data) = extensions.remove::<SpanData>() {
            span_data.end_time = Some(Utc::now());

            // Store to Gallifrey if enabled
            #[cfg(feature = "std")]
            if self.config.gallifrey_enabled {
                if let Some(store) = &self.store {
                    let store = Arc::clone(store);
                    let span_data_clone = span_data.clone();
                    tokio::spawn(async move {
                        let _ = store.record_span(span_data_clone).await;
                    });
                }
            }

            // Send to OTLP exporter if enabled
            if let Some(sender) = &self.otlp_sender {
                let _ = sender.try_send(span_data);
            }
        }
    }
}

/// Field visitor for collecting span attributes.
struct FieldVisitor<'a>(&'a mut HashMap<String, String>);

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

/// Event visitor for collecting event message and fields.
struct EventVisitor<'a> {
    fields: &'a mut HashMap<String, String>,
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for EventVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.message = format!("{value:?}");
        } else {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        } else {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}
