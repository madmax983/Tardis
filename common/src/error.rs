//! Error types for Tardis OS.
//!
//! Provides a unified error type used across all subsystems, with specific
//! variants for each component (Vortex, Gallifrey, Chronos, etc.).

use thiserror::Error;

/// The main error type for Tardis OS.
#[derive(Error, Debug)]
pub enum Error {
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
        false
    }

    /// Check if this error is a user error (vs system error).
    #[must_use]
    pub const fn is_user_error(&self) -> bool {
        matches!(self, Self::InvalidConfig(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::internal("test error");
        assert_eq!(err.to_string(), "internal error: test error");
    }

    #[test]
    fn error_is_recoverable() {
        assert!(!Error::Internal("test".to_string()).is_recoverable());
    }
}
