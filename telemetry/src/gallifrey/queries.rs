//! Temporal query helpers for telemetry.

use crate::gallifrey::store::{StoredEvent, StoredSpan};
use crate::types::{SpanId, TraceId};
use chrono::{DateTime, Utc};

/// Context around a specific timestamp.
///
/// Contains all telemetry data (spans and events) within a time window
/// centered on a specific moment.
#[derive(Debug, Clone)]
pub struct TelemetryContext {
    /// Center timestamp of the context.
    pub center: DateTime<Utc>,

    /// Window size in milliseconds (before and after center).
    pub window_ms: i64,

    /// Spans active during this window.
    pub spans: Vec<StoredSpan>,

    /// Events that occurred during this window.
    pub events: Vec<StoredEvent>,
}

impl TelemetryContext {
    /// Returns the start of the time window.
    #[must_use]
    pub fn start(&self) -> DateTime<Utc> {
        self.center - chrono::Duration::milliseconds(self.window_ms)
    }

    /// Returns the end of the time window.
    #[must_use]
    pub fn end(&self) -> DateTime<Utc> {
        self.center + chrono::Duration::milliseconds(self.window_ms)
    }

    /// Returns whether the context is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() && self.events.is_empty()
    }

    /// Returns the total number of items in the context.
    #[must_use]
    pub fn len(&self) -> usize {
        self.spans.len() + self.events.len()
    }
}

/// Query builder for telemetry data.
#[derive(Debug, Clone, Default)]
pub struct TelemetryQuery {
    /// Filter by trace ID.
    pub trace_id: Option<TraceId>,

    /// Filter by span ID.
    pub span_id: Option<SpanId>,

    /// Valid time filter (when the span was active).
    pub valid_at: Option<DateTime<Utc>>,

    /// Transaction time filter (when we knew about it).
    pub transaction_at: Option<DateTime<Utc>>,

    /// Time range filter.
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,

    /// Include historical versions.
    pub include_history: bool,

    /// Maximum results.
    pub limit: Option<usize>,
}

impl TelemetryQuery {
    /// Creates a new empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filters by trace ID.
    #[must_use]
    pub const fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Filters by span ID.
    #[must_use]
    pub const fn with_span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Filters to spans active at a specific time.
    #[must_use]
    pub const fn valid_at(mut self, time: DateTime<Utc>) -> Self {
        self.valid_at = Some(time);
        self
    }

    /// Filters to what we knew at a specific time.
    #[must_use]
    pub const fn transaction_at(mut self, time: DateTime<Utc>) -> Self {
        self.transaction_at = Some(time);
        self
    }

    /// Filters to a time range.
    #[must_use]
    pub const fn in_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.time_range = Some((from, to));
        self
    }

    /// Includes historical versions.
    #[must_use]
    pub const fn with_history(mut self) -> Self {
        self.include_history = true;
        self
    }

    /// Limits the number of results.
    #[must_use]
    pub const fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}
