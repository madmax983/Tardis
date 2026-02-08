//! OpenTelemetry export module.
//!
//! Provides OTLP (OpenTelemetry Protocol) export for traces and metrics.
//!
//! # Features
//!
//! - OTLP gRPC export for traces
//! - Async batching for performance
//! - Prometheus format for metrics
//!
//! # Example
//!
//! ```rust,ignore
//! use tardis_telemetry::export::OtlpConfig;
//! use tardis_telemetry::export::OtlpExporter;
//!
//! let config = OtlpConfig::new("http://localhost:4317");
//! let exporter = OtlpExporter::new(config)?;
//! // Spans are automatically batched and exported
//! ```

mod batch;
mod otlp;

pub use batch::BatchProcessor;
pub use otlp::{OtlpConfig, OtlpExporter};
