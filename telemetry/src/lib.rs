//! # Tardis Telemetry
//!
//! Full-stack observability for Tardis OS with temporal query support.
//!
//! This crate provides unified telemetry across the Tardis OS stack:
//!
//! - **Traces**: Request flows with timing via `tracing` integration
//! - **Metrics**: Counters, gauges, and histograms
//! - **Events**: Structured log entries with context
//!
//! ## Features
//!
//! - `std` (default): Userspace support with tracing, metrics, and Gallifrey storage
//! - `kernel`: no_std kernel support with ring buffer and serial output
//! - `otlp`: OpenTelemetry Protocol export to Jaeger/Grafana/Prometheus
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           External Tools                │
//! │    Jaeger / Grafana / Prometheus        │
//! └─────────────────▲───────────────────────┘
//!                   │ OTLP
//! ┌─────────────────┴───────────────────────┐
//! │          tardis-telemetry               │
//! │  TardisLayer → Gallifrey TelemetryStore │
//! │  MetricsRegistry → OTLP Exporter        │
//! └─────────────────▲───────────────────────┘
//!                   │ Ring Buffer
//! ┌─────────────────┴───────────────────────┐
//! │              Kernel                     │
//! │  KernelLogger → RingBuffer → Serial     │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use tardis_telemetry::{init, TelemetryConfig};
//!
//! // Initialize telemetry
//! let config = TelemetryConfig::default();
//! let handle = init(config)?;
//!
//! // Use tracing macros as normal
//! tracing::info!("System started");
//!
//! // Record metrics
//! use tardis_telemetry::metrics::{counter, histogram};
//! counter!("requests_total").inc();
//! histogram!("latency_ms").record(42);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::unused_async)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::non_std_lazy_statics)]
#![allow(clippy::elidable_lifetime_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_fields_in_debug)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::if_then_some_else_none)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::unused_self)]
#![allow(clippy::expect_used)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::uninlined_format_args)]
#![allow(dead_code)]

extern crate alloc;

pub mod types;

#[cfg(feature = "kernel")]
pub mod kernel;

#[cfg(feature = "std")]
pub mod userspace;

#[cfg(feature = "std")]
pub mod gallifrey;

#[cfg(feature = "otlp")]
pub mod export;

mod error;

// Re-exports
pub use error::{TelemetryError, TelemetryResult};
pub use types::{EventType, Level, SpanId, Subsystem, TelemetryEntry, TraceId};

#[cfg(feature = "std")]
pub use userspace::{TelemetryConfig, TelemetryHandle, init};

#[cfg(feature = "std")]
pub mod metrics {
    //! Metrics collection utilities.
    //!
    //! Provides thread-safe counters, gauges, and histograms.
    //!
    //! Use the [`counter!`], [`gauge!`], and [`histogram!`] macros
    //! at the crate root for convenient access.

    pub use crate::userspace::metrics::{Counter, Gauge, Histogram, METRICS, MetricsRegistry};
}

// Note: counter!, gauge!, histogram! macros are automatically exported
// at the crate root via #[macro_export] in userspace/metrics.rs
