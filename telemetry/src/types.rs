//! Core telemetry types.
//!
//! All types in this module are `no_std` compatible and can be used
//! in both kernel and userspace contexts.

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};

/// 128-bit trace identifier (W3C Trace Context compatible).
///
/// Uniquely identifies a distributed trace across the system.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct TraceId([u8; 16]);

impl TraceId {
    /// A zero/null trace ID indicating no trace context.
    pub const NONE: Self = Self([0; 16]);

    /// Creates a new trace ID from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes of this trace ID.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Checks if this is a null/zero trace ID.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.0 == [0; 16]
    }

    /// Generates a new random trace ID.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let mut bytes = [0u8; 16];
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        // Use timestamp for first 8 bytes
        bytes[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);

        // Use simple counter/random for last 8 bytes
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        bytes[8..].copy_from_slice(&count.to_le_bytes());

        Self(bytes)
    }
}

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TraceId({self})")
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// 64-bit span identifier.
///
/// Uniquely identifies a span within a trace.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct SpanId([u8; 8]);

impl SpanId {
    /// A zero/null span ID indicating no span context.
    pub const NONE: Self = Self([0; 8]);

    /// Creates a new span ID from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes of this span ID.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    /// Checks if this is a null/zero span ID.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.0 == [0; 8]
    }

    /// Generates a new span ID.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn generate() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(id.to_le_bytes())
    }
}

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpanId({self})")
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Telemetry severity level.
///
/// Compatible with both `log` and `tracing` crate levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Level {
    /// Verbose debugging information.
    Trace = 0,
    /// Development/debugging information.
    Debug = 1,
    /// Normal operational information.
    Info = 2,
    /// Potential issues that don't prevent operation.
    Warn = 3,
    /// Errors that affect operation.
    Error = 4,
}

impl Level {
    /// Returns the level as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "std")]
impl From<tracing::Level> for Level {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}

/// Subsystem identifier for routing and filtering.
///
/// Identifies which component of Tardis OS generated the telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum Subsystem {
    // Kernel subsystems (0-99)
    /// Core kernel functionality.
    Kernel = 0,
    /// Memory management.
    Memory = 1,
    /// Process scheduler.
    Scheduler = 2,
    /// Interrupt handling.
    Interrupt = 3,
    /// Syscall interface.
    Syscall = 4,
    /// Boot sequence.
    Boot = 5,

    // Userspace services (100-199)
    /// Vortex LLM inference engine.
    Vortex = 100,
    /// Gallifrey temporal database.
    Gallifrey = 101,
    /// Chronos RAG orchestration.
    Chronos = 102,
    /// Shell user interface.
    Shell = 103,
    /// Telemetry subsystem itself.
    Telemetry = 104,

    // External/Unknown (200+)
    /// Unknown or external subsystem.
    Unknown = 255,
}

impl Subsystem {
    /// Returns the subsystem as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Memory => "memory",
            Self::Scheduler => "scheduler",
            Self::Interrupt => "interrupt",
            Self::Syscall => "syscall",
            Self::Boot => "boot",
            Self::Vortex => "vortex",
            Self::Gallifrey => "gallifrey",
            Self::Chronos => "chronos",
            Self::Shell => "shell",
            Self::Telemetry => "telemetry",
            Self::Unknown => "unknown",
        }
    }

    /// Parses a subsystem from a target string.
    #[must_use]
    pub fn from_target(target: &str) -> Self {
        let target_lower = target.to_lowercase();

        if target_lower.contains("kernel") {
            Self::Kernel
        } else if target_lower.contains("memory") || target_lower.contains("heap") {
            Self::Memory
        } else if target_lower.contains("scheduler") || target_lower.contains("process") {
            Self::Scheduler
        } else if target_lower.contains("interrupt") || target_lower.contains("irq") {
            Self::Interrupt
        } else if target_lower.contains("syscall") {
            Self::Syscall
        } else if target_lower.contains("boot") {
            Self::Boot
        } else if target_lower.contains("vortex") {
            Self::Vortex
        } else if target_lower.contains("gallifrey") {
            Self::Gallifrey
        } else if target_lower.contains("chronos") {
            Self::Chronos
        } else if target_lower.contains("shell") || target_lower.contains("tardis") {
            Self::Shell
        } else if target_lower.contains("telemetry") {
            Self::Telemetry
        } else {
            Self::Unknown
        }
    }
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Event type within a subsystem.
///
/// Provides fine-grained categorization of telemetry events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum EventType {
    // Lifecycle events (0-99)
    /// Span started.
    SpanStart = 0,
    /// Span ended.
    SpanEnd = 1,
    /// Log message.
    Log = 2,
    /// Metric sample.
    Metric = 3,

    // Kernel events (100-199)
    /// Boot sequence event.
    Boot = 100,
    /// Kernel panic.
    Panic = 101,
    /// Syscall entry.
    SyscallEntry = 102,
    /// Syscall exit.
    SyscallExit = 103,
    /// Page fault.
    PageFault = 104,
    /// Interrupt received.
    InterruptReceived = 105,
    /// Context switch.
    ContextSwitch = 106,

    // Vortex events (200-299)
    /// Model loading started.
    ModelLoad = 200,
    /// Model unloaded.
    ModelUnload = 201,
    /// Inference started.
    InferenceStart = 202,
    /// Inference completed.
    InferenceEnd = 203,
    /// Token generated.
    TokenGenerated = 204,
    /// Embedding computed.
    EmbeddingComputed = 205,

    // Gallifrey events (300-399)
    /// Query started.
    QueryStart = 300,
    /// Query completed.
    QueryEnd = 301,
    /// Entity inserted.
    EntityInsert = 302,
    /// Entity updated.
    EntityUpdate = 303,
    /// Time travel query.
    TimeTravel = 304,
    /// Snapshot taken.
    SnapshotTaken = 305,

    // Chronos events (400-499)
    /// RAG query started.
    RagQuery = 400,
    /// Retrieval operation.
    Retrieval = 401,
    /// Context augmentation.
    Augmentation = 402,
    /// Memory consolidation.
    Consolidation = 403,

    // Shell events (500-599)
    /// User input received.
    UserInput = 500,
    /// Intent classified.
    IntentClassified = 501,
    /// Response generated.
    ResponseGenerated = 502,

    // Unknown
    /// Unknown event type.
    Unknown = 65535,
}

impl EventType {
    /// Returns the event type as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SpanStart => "span_start",
            Self::SpanEnd => "span_end",
            Self::Log => "log",
            Self::Metric => "metric",
            Self::Boot => "boot",
            Self::Panic => "panic",
            Self::SyscallEntry => "syscall_entry",
            Self::SyscallExit => "syscall_exit",
            Self::PageFault => "page_fault",
            Self::InterruptReceived => "interrupt",
            Self::ContextSwitch => "context_switch",
            Self::ModelLoad => "model_load",
            Self::ModelUnload => "model_unload",
            Self::InferenceStart => "inference_start",
            Self::InferenceEnd => "inference_end",
            Self::TokenGenerated => "token_generated",
            Self::EmbeddingComputed => "embedding_computed",
            Self::QueryStart => "query_start",
            Self::QueryEnd => "query_end",
            Self::EntityInsert => "entity_insert",
            Self::EntityUpdate => "entity_update",
            Self::TimeTravel => "time_travel",
            Self::SnapshotTaken => "snapshot_taken",
            Self::RagQuery => "rag_query",
            Self::Retrieval => "retrieval",
            Self::Augmentation => "augmentation",
            Self::Consolidation => "consolidation",
            Self::UserInput => "user_input",
            Self::IntentClassified => "intent_classified",
            Self::ResponseGenerated => "response_generated",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

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

/// Metric value types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Monotonically increasing counter.
    Counter(u64),
    /// Value that can go up or down.
    Gauge(i64),
    /// Distribution of values with buckets.
    Histogram {
        /// Sum of all recorded values.
        sum: f64,
        /// Count of recorded values.
        count: u64,
        /// Bucket counts (index corresponds to predefined bucket boundaries).
        buckets: alloc::vec::Vec<u64>,
    },
}

/// A recorded metric sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    /// Metric name.
    pub name: String,
    /// Originating subsystem.
    pub subsystem: Subsystem,
    /// Timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Metric value.
    pub value: MetricValue,
    /// Labels as key-value pairs.
    pub labels: alloc::vec::Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_display() {
        let id = TraceId::from_bytes([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ]);
        assert_eq!(id.to_string(), "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn span_id_display() {
        let id = SpanId::from_bytes([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
        assert_eq!(id.to_string(), "0123456789abcdef");
    }

    #[test]
    fn level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn subsystem_from_target() {
        assert_eq!(
            Subsystem::from_target("tardis_vortex::inference"),
            Subsystem::Vortex
        );
        assert_eq!(
            Subsystem::from_target("tardis_gallifrey::stores"),
            Subsystem::Gallifrey
        );
        assert_eq!(
            Subsystem::from_target("memory::allocator"),
            Subsystem::Memory
        );
        assert_eq!(
            Subsystem::from_target("tardis_kernel::core"),
            Subsystem::Kernel
        );
        assert_eq!(Subsystem::from_target("random_crate"), Subsystem::Unknown);
    }
}
