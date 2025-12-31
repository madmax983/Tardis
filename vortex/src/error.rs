//! Error types for Vortex.

use thiserror::Error;

/// Vortex-specific errors.
#[derive(Error, Debug)]
pub enum VortexError {
    /// Model file not found.
    #[error("model not found: {path}")]
    ModelNotFound {
        /// Path to the model
        path: String,
    },

    /// Failed to load model.
    #[error("failed to load model: {0}")]
    LoadFailed(String),

    /// Invalid model architecture.
    #[error("unsupported architecture: {0}")]
    UnsupportedArchitecture(String),

    /// Invalid model handle.
    #[error("invalid model handle: {0}")]
    InvalidHandle(u64),

    /// Model not loaded.
    #[error("model not loaded: {0}")]
    ModelNotLoaded(u64),

    /// Inference failed.
    #[error("inference failed: {0}")]
    InferenceFailed(String),

    /// Tokenization error.
    #[error("tokenization error: {0}")]
    TokenizationError(String),

    /// Device error.
    #[error("device error: {0}")]
    DeviceError(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// Candle error.
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for Vortex operations.
pub type VortexResult<T> = Result<T, VortexError>;
