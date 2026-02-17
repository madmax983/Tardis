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

    /// Entity already exists (current version).
    #[error("entity already exists: {0}")]
    EntityAlreadyExists(String),

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

impl From<GallifreyError> for tardis_common::Error {
    fn from(err: GallifreyError) -> Self {
        match err {
            GallifreyError::QueryParseError(msg) => Self::QueryParseError(msg),
            GallifreyError::QueryExecutionFailed(msg) => Self::QueryExecutionFailed(msg),
            GallifreyError::EntityNotFound(msg) => Self::EntityNotFound(msg),
            GallifreyError::EntityAlreadyExists(msg) => {
                Self::Internal(format!("Entity already exists: {}", msg))
            }
            GallifreyError::InvalidTemporalReference(msg) => Self::InvalidTemporalReference(msg),
            GallifreyError::TimeTravelFailed(msg) => Self::TimeTravelFailed { reason: msg },
            GallifreyError::Serialization(e) => Self::Serialization(e),
            GallifreyError::Io(e) => Self::Io(e),
            _ => Self::Internal(err.to_string()),
        }
    }
}
