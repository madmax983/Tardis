//! Wire format and ring buffer entries.
//!
//! Provides compact structures for telemetry transmission.
//! `no_std` compatible.

use crate::log::Level;
use crate::meta::{EventType, Subsystem};
use crate::trace::{SpanId, TraceId};
use serde::{Deserialize, Serialize};

/// Compact telemetry entry for kernel ring buffer.
///
/// This structure is designed for efficient storage in a lock-free ring buffer:
/// - Fixed-size header for fast indexing
/// - Compact representation to minimize memory bandwidth
/// - Aligned for cache-friendly access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct TelemetryEntry {
    /// Nanoseconds since boot (or epoch in userspace).
    pub timestamp_ns: u64,
    /// Severity level.
    pub level: Level,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Event type.
    pub event_type: EventType,
    /// Current span ID.
    pub span_id: SpanId,
    /// Trace ID for distributed tracing.
    pub trace_id: TraceId,
    /// Parent span ID (NONE if root).
    pub parent_span_id: SpanId,
    /// Length of the payload that follows.
    pub payload_len: u16,
}

impl TelemetryEntry {
    /// Creates a new telemetry entry with the current timestamp.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn new(
        level: Level,
        subsystem: Subsystem,
        event_type: EventType,
        trace_id: TraceId,
        span_id: SpanId,
        parent_span_id: SpanId,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        #[allow(clippy::cast_possible_truncation)]
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        Self {
            timestamp_ns,
            level,
            subsystem,
            event_type,
            span_id,
            trace_id,
            parent_span_id,
            payload_len: 0,
        }
    }

    /// Creates a log entry.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn log(level: Level, subsystem: Subsystem) -> Self {
        Self::new(
            level,
            subsystem,
            EventType::Log,
            TraceId::NONE,
            SpanId::NONE,
            SpanId::NONE,
        )
    }
}
