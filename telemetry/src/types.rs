//! Core telemetry types.
//!
//! This module re-exports types from specialized modules for backward compatibility.
//! All types in this module are `no_std` compatible.

pub use crate::log::Level;
pub use crate::meta::{EventType, Subsystem};
pub use crate::metrics_types::{MetricSample, MetricValue};
pub use crate::trace::{SpanId, TraceId};
pub use crate::wire::TelemetryEntry;
