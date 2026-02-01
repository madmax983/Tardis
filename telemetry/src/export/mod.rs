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
//! use tardis_telemetry::export::OtlpExporter;
//!
//! let exporter = OtlpExporter::new("http://localhost:4317").await?;
//! // Spans are automatically batched and exported
//! ```

mod batch;
mod otlp;

pub use batch::BatchProcessor;
pub use otlp::{OtlpConfig, OtlpExporter};
