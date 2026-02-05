//! Distributed tracing primitives.
//!
//! Provides unique identifiers for traces and spans, compatible with W3C Trace Context.
//! `no_std` compatible.

use core::fmt;
use serde::{Deserialize, Serialize};

/// 128-bit trace identifier (W3C Trace Context compatible).
///
/// Uniquely identifies a distributed trace across the system.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct TraceId([u8; 16]);

impl TraceId {
    /// A zero/null trace ID indicating no trace context.
    pub const NONE: Self = Self([0; 16]);

    /// Creates a new trace ID from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes of this trace ID.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Checks if this is a null/zero trace ID.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.0 == [0; 16]
    }

    /// Generates a new random trace ID.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Use simple counter/random for last 8 bytes
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let mut bytes = [0u8; 16];
        #[allow(clippy::cast_possible_truncation)]
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        // Use timestamp for first 8 bytes
        bytes[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);

        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        bytes[8..].copy_from_slice(&count.to_le_bytes());

        Self(bytes)
    }
}

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TraceId({self})")
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// 64-bit span identifier.
///
/// Uniquely identifies a span within a trace.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct SpanId([u8; 8]);

impl SpanId {
    /// A zero/null span ID indicating no span context.
    pub const NONE: Self = Self([0; 8]);

    /// Creates a new span ID from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes of this span ID.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    /// Checks if this is a null/zero span ID.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.0 == [0; 8]
    }

    /// Generates a new span ID.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn generate() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self(id.to_le_bytes())
    }
}

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpanId({self})")
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn trace_id_display() {
        let id = TraceId::from_bytes([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ]);
        assert_eq!(id.to_string(), "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn span_id_display() {
        let id = SpanId::from_bytes([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
        assert_eq!(id.to_string(), "0123456789abcdef");
    }
}
