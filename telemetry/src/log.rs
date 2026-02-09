//! Logging primitives.
//!
//! Provides severity levels and integration with logging frameworks.
//! `no_std` compatible.

use core::fmt;
use serde::{Deserialize, Serialize};

/// Telemetry severity level.
///
/// Compatible with both `log` and `tracing` crate levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Level {
    /// Verbose debugging information.
    Trace = 0,
    /// Development/debugging information.
    Debug = 1,
    /// Normal operational information.
    Info = 2,
    /// Potential issues that don't prevent operation.
    Warn = 3,
    /// Errors that affect operation.
    Error = 4,
}

impl Level {
    /// Returns the level as a static string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Trace,
            1 => Self::Debug,
            3 => Self::Warn,
            4 => Self::Error,
            // Map invalid levels (and 2/Info) to Info to avoid noise but prevent UB
            _ => Self::Info,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "std")]
impl From<tracing::Level> for Level {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }
}
