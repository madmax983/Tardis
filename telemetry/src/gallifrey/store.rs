//! Telemetry store backed by Gallifrey.

use crate::error::TelemetryResult;
use crate::types::{MetricSample, SpanId, TraceId};
use crate::userspace::layer::{EventData, SpanData};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tardis_common::EntityId;

/// Stored span record.
#[derive(Debug, Clone)]
pub struct StoredSpan {
    /// Entity ID in Gallifrey.
    pub entity_id: EntityId,
    /// Original span data.
    pub data: SpanData,
}

/// Stored event record.
#[derive(Debug, Clone)]
pub struct StoredEvent {
    /// Entity ID in Gallifrey.
    pub entity_id: EntityId,
    /// Original event data.
    pub data: EventData,
}

/// Telemetry storage backed by Gallifrey.
///
/// This store maintains spans and events as entities in Gallifrey's
/// knowledge graph, enabling temporal queries.
pub struct TelemetryStore {
    /// Spans indexed by span ID.
    spans: RwLock<HashMap<SpanId, StoredSpan>>,

    /// Spans indexed by trace ID.
    traces: RwLock<HashMap<TraceId, Vec<SpanId>>>,

    /// Events indexed by entity ID.
    events: RwLock<Vec<StoredEvent>>,

    /// Metrics history.
    metrics: RwLock<Vec<MetricSample>>,
}

impl TelemetryStore {
    /// Creates a new telemetry store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spans: RwLock::new(HashMap::new()),
            traces: RwLock::new(HashMap::new()),
            events: RwLock::new(Vec::new()),
            metrics: RwLock::new(Vec::new()),
        }
    }

    /// Records a completed span.
    pub async fn record_span(&self, span: SpanData) -> TelemetryResult<EntityId> {
        let entity_id = EntityId::new();
        let span_id = span.span_id;
        let trace_id = span.trace_id;

        let stored = StoredSpan {
            entity_id,
            data: span,
        };

        // Store by span ID
        self.spans.write().insert(span_id, stored);

        // Index by trace ID
        self.traces
            .write()
            .entry(trace_id)
            .or_default()
            .push(span_id);

        Ok(entity_id)
    }

    /// Records an event.
    pub async fn record_event(&self, event: EventData) -> TelemetryResult<EntityId> {
        let entity_id = EntityId::new();

        let stored = StoredEvent {
            entity_id,
            data: event,
        };

        self.events.write().push(stored);

        Ok(entity_id)
    }

    /// Records a metric sample.
    pub async fn record_metric(&self, metric: MetricSample) -> TelemetryResult<()> {
        self.metrics.write().push(metric);
        Ok(())
    }

    /// Gets a span by its ID.
    #[must_use]
    pub fn get_span(&self, span_id: SpanId) -> Option<StoredSpan> {
        self.spans.read().get(&span_id).cloned()
    }

    /// Gets all spans for a trace.
    #[must_use]
    pub fn get_trace_spans(&self, trace_id: TraceId) -> Vec<StoredSpan> {
        let traces = self.traces.read();
        let spans = self.spans.read();

        traces
            .get(&trace_id)
            .map(|span_ids| {
                span_ids
                    .iter()
                    .filter_map(|id| spans.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Finds spans active at a specific time.
    #[must_use]
    pub fn spans_at(
        &self,
        valid_time: DateTime<Utc>,
        _transaction_time: Option<DateTime<Utc>>,
    ) -> Vec<StoredSpan> {
        self.spans
            .read()
            .values()
            .filter(|stored| {
                let start = stored.data.start_time;
                let end = stored.data.end_time.unwrap_or_else(Utc::now);
                valid_time >= start && valid_time <= end
            })
            .cloned()
            .collect()
    }

    /// Finds spans in a time range.
    #[must_use]
    pub fn spans_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<StoredSpan> {
        self.spans
            .read()
            .values()
            .filter(|stored| {
                let start = stored.data.start_time;
                let end = stored.data.end_time.unwrap_or_else(Utc::now);
                // Span overlaps with range
                start <= to && end >= from
            })
            .cloned()
            .collect()
    }

    /// Finds events in a time range.
    #[must_use]
    pub fn events_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<StoredEvent> {
        self.events
            .read()
            .iter()
            .filter(|stored| stored.data.timestamp >= from && stored.data.timestamp <= to)
            .cloned()
            .collect()
    }

    /// Gets metric history for a specific metric.
    #[must_use]
    pub fn metric_history(
        &self,
        metric_name: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<MetricSample> {
        let from_ns = from.timestamp_nanos_opt().unwrap_or(0) as u64;
        let to_ns = to.timestamp_nanos_opt().unwrap_or(i64::MAX) as u64;

        self.metrics
            .read()
            .iter()
            .filter(|m| {
                m.name == metric_name && m.timestamp_ns >= from_ns && m.timestamp_ns <= to_ns
            })
            .cloned()
            .collect()
    }

    /// Gets context around a specific timestamp.
    ///
    /// Returns all spans and events within the specified time window.
    #[must_use]
    pub fn context_around(
        &self,
        timestamp: DateTime<Utc>,
        window_ms: i64,
    ) -> super::queries::TelemetryContext {
        let window = chrono::Duration::milliseconds(window_ms);
        let from = timestamp - window;
        let to = timestamp + window;

        let spans = self.spans_in_range(from, to);
        let events = self.events_in_range(from, to);

        super::queries::TelemetryContext {
            center: timestamp,
            window_ms,
            spans,
            events,
        }
    }

    /// Returns the total number of stored spans.
    #[must_use]
    pub fn span_count(&self) -> usize {
        self.spans.read().len()
    }

    /// Returns the total number of stored events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.read().len()
    }

    /// Clears all stored telemetry data.
    pub fn clear(&self) {
        self.spans.write().clear();
        self.traces.write().clear();
        self.events.write().clear();
        self.metrics.write().clear();
    }
}

impl Default for TelemetryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TelemetryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryStore")
            .field("span_count", &self.span_count())
            .field("event_count", &self.event_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Level, Subsystem};
    use std::collections::HashMap;
    use std::time::Instant;

    #[tokio::test]
    async fn store_and_retrieve_span() {
        let store = TelemetryStore::new();

        let span = SpanData {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_id: None,
            name: "test_span".to_string(),
            subsystem: Subsystem::Vortex,
            start_instant: Instant::now(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            attributes: HashMap::new(),
            level: Level::Info,
        };

        let span_id = span.span_id;
        store.record_span(span).await.unwrap();

        let retrieved = store.get_span(span_id).unwrap();
        assert_eq!(retrieved.data.name, "test_span");
    }

    #[test]
    fn spans_at_time() {
        let store = TelemetryStore::new();

        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let future = now + chrono::Duration::hours(1);

        // Add a span that spans the current time
        let span = SpanData {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_id: None,
            name: "active_span".to_string(),
            subsystem: Subsystem::Vortex,
            start_instant: Instant::now(),
            start_time: past,
            end_time: Some(future),
            attributes: HashMap::new(),
            level: Level::Info,
        };

        store.spans.write().insert(
            span.span_id,
            StoredSpan {
                entity_id: EntityId::new(),
                data: span,
            },
        );

        let active = store.spans_at(now, None);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].data.name, "active_span");
    }
}
