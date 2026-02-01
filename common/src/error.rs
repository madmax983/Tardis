//! Error types for Tardis OS.
//!
//! Provides a unified error type used across all subsystems, with specific
//! variants for each component (Vortex, Gallifrey, Chronos, etc.).

use thiserror::Error;

/// The main error type for Tardis OS.
#[derive(Error, Debug)]
pub enum Error {
    // ========================================================================
    // Vortex (LLM) Errors
    // ========================================================================
    /// Model not found at specified path
    #[error("model not found: {path}")]
    ModelNotFound {
        /// Path where model was expected
        path: String,
    },

    /// Model failed to load
    #[error("failed to load model '{name}': {reason}")]
    ModelLoadFailed {
        /// Model name or path
        name: String,
        /// Failure reason
        reason: String,
    },

    /// Inference failed
    #[error("inference failed: {reason}")]
    InferenceFailed {
        /// Failure reason
        reason: String,
    },

    /// Invalid model handle
    #[error("invalid model handle: {0}")]
    InvalidModelHandle(u64),

    /// Tokenization error
    #[error("tokenization failed: {0}")]
    TokenizationFailed(String),

    // ========================================================================
    // Gallifrey (Database) Errors
    // ========================================================================
    /// Query parsing failed
    #[error("query parse error: {0}")]
    QueryParseError(String),

    /// Query execution failed
    #[error("query execution failed: {0}")]
    QueryExecutionFailed(String),

    /// Entity not found
    #[error("entity not found: {0}")]
    EntityNotFound(String),

    /// Invalid temporal reference
    #[error("invalid temporal reference: {0}")]
    InvalidTemporalReference(String),

    /// Time travel failed
    #[error("time travel failed: {reason}")]
    TimeTravelFailed {
        /// Failure reason
        reason: String,
    },

    // ========================================================================
    // Chronos (RAG) Errors
    // ========================================================================
    /// Retrieval failed
    #[error("retrieval failed: {0}")]
    RetrievalFailed(String),

    /// Context assembly failed
    #[error("context assembly failed: {0}")]
    ContextAssemblyFailed(String),

    /// Memory storage failed
    #[error("memory storage failed: {0}")]
    MemoryStorageFailed(String),

    // ========================================================================
    // Shell Errors
    // ========================================================================
    /// Command not found
    #[error("command not found: {0}")]
    CommandNotFound(String),

    /// Command execution failed
    #[error("command failed: {command}: {reason}")]
    CommandFailed {
        /// Command that failed
        command: String,
        /// Failure reason
        reason: String,
    },

    /// Session error
    #[error("session error: {0}")]
    SessionError(String),

    // ========================================================================
    // General Errors
    // ========================================================================
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Resource exhausted
    #[error("resource exhausted: {resource}")]
    ResourceExhausted {
        /// Name of exhausted resource
        resource: String,
    },

    /// Operation not supported
    #[error("operation not supported: {0}")]
    NotSupported(String),

    /// Internal error (should not happen)
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type alias using Tardis Error.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new internal error.
    #[must_use]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ModelNotFound { .. }
                | Self::EntityNotFound(_)
                | Self::CommandNotFound(_)
                | Self::InvalidTemporalReference(_)
        )
    }

    /// Check if this error is a user error (vs system error).
    #[must_use]
    pub const fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::QueryParseError(_)
                | Self::InvalidConfig(_)
                | Self::InvalidTemporalReference(_)
                | Self::CommandNotFound(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::ModelNotFound {
            path: "/models/llama.gguf".to_string(),
        };
        assert_eq!(err.to_string(), "model not found: /models/llama.gguf");
    }

    #[test]
    fn error_is_recoverable() {
        assert!(Error::ModelNotFound {
            path: "test".to_string()
        }
        .is_recoverable());
        assert!(!Error::Internal("test".to_string()).is_recoverable());
    }
}
