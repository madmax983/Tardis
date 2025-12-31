//! Error types for Gallifrey.

use thiserror::Error;

/// Gallifrey-specific errors.
#[derive(Error, Debug)]
pub enum GallifreyError {
    /// Query parsing failed.
    #[error("query parse error: {0}")]
    QueryParseError(String),

    /// Query execution failed.
    #[error("query execution failed: {0}")]
    QueryExecutionFailed(String),

    /// Entity not found.
    #[error("entity not found: {0}")]
    EntityNotFound(String),

    /// Relationship not found.
    #[error("relationship not found: {0}")]
    RelationshipNotFound(String),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Snapshot not found.
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(String),

    /// Invalid temporal reference.
    #[error("invalid temporal reference: {0}")]
    InvalidTemporalReference(String),

    /// Time travel failed.
    #[error("time travel failed: {0}")]
    TimeTravelFailed(String),

    /// Storage error.
    #[error("storage error: {0}")]
    StorageError(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for Gallifrey operations.
pub type GallifreyResult<T> = Result<T, GallifreyError>;
