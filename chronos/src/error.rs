//! Error types for Chronos.

use thiserror::Error;

/// Chronos-specific errors.
#[derive(Error, Debug)]
pub enum ChronosError {
    /// Query analysis failed.
    #[error("query analysis failed: {0}")]
    QueryAnalysisFailed(String),

    /// Retrieval failed.
    #[error("retrieval failed: {0}")]
    RetrievalFailed(String),

    /// Context assembly failed.
    #[error("context assembly failed: {0}")]
    ContextAssemblyFailed(String),

    /// Inference failed.
    #[error("inference failed: {0}")]
    InferenceFailed(String),

    /// Memory operation failed.
    #[error("memory operation failed: {0}")]
    MemoryFailed(String),

    /// Invalid temporal reference.
    #[error("invalid temporal reference: {0}")]
    InvalidTemporalReference(String),

    /// Vortex error.
    #[error("vortex error: {0}")]
    Vortex(#[from] tardis_vortex::VortexError),

    /// Gallifrey error.
    #[error("gallifrey error: {0}")]
    Gallifrey(#[from] tardis_gallifrey::GallifreyError),
}

/// Result type for Chronos operations.
pub type ChronosResult<T> = Result<T, ChronosError>;
