//! Userspace telemetry module.
//!
//! Provides full-featured telemetry for Tardis OS userspace services:
//!
//! - [`TardisLayer`]: Custom `tracing::Layer` for span and event capture
//! - [`metrics`]: Thread-safe counters, gauges, and histograms
//! - [`init`]: Initialize the complete telemetry stack
//!
//! # Usage
//!
//! ```rust,ignore
//! use tardis_telemetry::{init, TelemetryConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize telemetry
//!     let config = TelemetryConfig::default();
//!     let handle = init(config)?;
//!
//!     // Use tracing macros as normal
//!     tracing::info!("Application started");
//!
//!     // Record metrics
//!     use tardis_telemetry::metrics::counter;
//!     counter!("requests_total").inc();
//!
//!     Ok(())
//! }
//! ```

mod drainer;
pub mod layer;
pub mod metrics;
pub mod subscriber;

pub use layer::TardisLayer;
pub use metrics::{Counter, Gauge, Histogram, MetricsRegistry};
pub use subscriber::{init, TelemetryConfig, TelemetryHandle};
