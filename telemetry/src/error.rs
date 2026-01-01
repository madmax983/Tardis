//! Telemetry error types.

use thiserror::Error;

/// Telemetry errors.
#[derive(Debug, Error)]
pub enum TelemetryError {
    /// Ring buffer is full, event was dropped.
    #[error("ring buffer full, event dropped")]
    RingBufferFull,

    /// Serial port initialization failed.
    #[error("serial port initialization failed: {0}")]
    SerialInit(alloc::string::String),

    /// Subscriber initialization failed.
    #[error("subscriber initialization failed: {0}")]
    SubscriberInit(alloc::string::String),

    /// Gallifrey storage error.
    #[error("gallifrey storage error: {0}")]
    #[cfg(feature = "std")]
    GallifreyError(#[from] tardis_gallifrey::error::GallifreyError),

    /// OpenTelemetry export error.
    #[error("OTLP export error: {0}")]
    #[cfg(feature = "otlp")]
    OtlpError(alloc::string::String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(alloc::string::String),

    /// I/O error.
    #[error("I/O error: {0}")]
    #[cfg(feature = "std")]
    Io(#[from] std::io::Error),
}

/// Result type for telemetry operations.
pub type TelemetryResult<T> = Result<T, TelemetryError>;
