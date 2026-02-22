//! Distributed tracing primitives.
//!
//! # Overview
//!
//! Distributed tracing allows you to track the propagation of a request across service boundaries.
//! This module provides the core identifiers used to correlate logs and metrics:
//!
//! - [`TraceId`]: A 128-bit unique identifier for a whole trace (a complete request lifecycle).
//! - [`SpanId`]: A 64-bit unique identifier for a single unit of work (a span) within a trace.
//!
//! # W3C Trace Context
//!
//! These identifiers are designed to be compatible with the [W3C Trace Context](https://www.w3.org/TR/trace-context/) specification,
//! ensuring interoperability with tools like OpenTelemetry, Jaeger, and Zipkin.
//!
//! # Usage
//!
//! ```rust
//! use tardis_telemetry::trace::{TraceId, SpanId};
//!
//! // Create IDs from raw bytes (e.g., received from a network header)
//! let trace_id = TraceId::from_bytes([
//!     0x4b, 0xf9, 0x2f, 0x35, 0x77, 0xb3, 0x4d, 0xa6,
//!     0xa3, 0xce, 0x92, 0x9d, 0x0e, 0x0e, 0x47, 0x36,
//! ]);
//!
//! let span_id = SpanId::from_bytes([
//!     0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7,
//! ]);
//!
//! println!("Trace: {}", trace_id);
//! println!("Span: {}", span_id);
//! ```
//!
//! `no_std` compatible.

use core::fmt;
use serde::{Deserialize, Serialize};

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

/// 128-bit trace identifier (W3C Trace Context compatible).
///
/// Uniquely identifies a distributed trace across the system. A trace represents
/// a single operation (like a user request) that may traverse multiple services or threads.
///
/// The ID consists of a 16-byte array, typically serialized as a 32-character hexadecimal string.
///
/// # Examples
///
/// Creating a `TraceId` from a byte array:
///
/// ```
/// use tardis_telemetry::trace::TraceId;
///
/// let bytes = [1u8; 16];
/// let trace_id = TraceId::from_bytes(bytes);
/// assert!(!trace_id.is_none());
/// ```
///
/// Generating a random `TraceId` (requires `std` feature):
///
/// ```
/// # #[cfg(feature = "std")]
/// # {
/// use tardis_telemetry::trace::TraceId;
///
/// let trace_id = TraceId::generate();
/// println!("New trace started: {}", trace_id);
/// # }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct TraceId([u8; 16]);

impl TraceId {
    /// A zero/null trace ID indicating no trace context.
    ///
    /// This is used when a valid trace ID is not available or required.
    pub const NONE: Self = Self([0; 16]);

    /// Creates a new trace ID from raw bytes.
    ///
    /// This is `const` fn, allowing it to be used in static initializers.
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
    /// Formats the trace ID as a 32-character hexadecimal string.
    ///
    /// This implementation is optimized to avoid memory allocation and formatting overhead,
    /// writing directly to a stack-allocated buffer.
    #[allow(unsafe_code)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 32];
        for (i, &byte) in self.0.iter().enumerate() {
            buf[i * 2] = HEX_CHARS[(byte >> 4) as usize];
            buf[i * 2 + 1] = HEX_CHARS[(byte & 0x0f) as usize];
        }
        // SAFETY: buf is filled only with ASCII hex characters from HEX_CHARS
        let s = unsafe { core::str::from_utf8_unchecked(&buf) };
        f.write_str(s)
    }
}

/// 64-bit span identifier.
///
/// Uniquely identifies a span within a trace. A span represents a logical unit of work,
/// such as a function call, a database query, or a network request.
///
/// The ID consists of an 8-byte array, typically serialized as a 16-character hexadecimal string.
///
/// # Examples
///
/// Creating a `SpanId` from a byte array:
///
/// ```
/// use tardis_telemetry::trace::SpanId;
///
/// let bytes = [0xAB; 8];
/// let span_id = SpanId::from_bytes(bytes);
/// assert_eq!(span_id.to_string(), "abababababababab");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct SpanId([u8; 8]);

impl SpanId {
    /// A zero/null span ID indicating no span context.
    pub const NONE: Self = Self([0; 8]);

    /// Creates a new span ID from raw bytes.
    ///
    /// This is `const` fn, allowing it to be used in static initializers.
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
    /// Formats the span ID as a 16-character hexadecimal string.
    ///
    /// This implementation is optimized to avoid memory allocation and formatting overhead,
    /// writing directly to a stack-allocated buffer.
    #[allow(unsafe_code)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 16];
        for (i, &byte) in self.0.iter().enumerate() {
            buf[i * 2] = HEX_CHARS[(byte >> 4) as usize];
            buf[i * 2 + 1] = HEX_CHARS[(byte & 0x0f) as usize];
        }
        // SAFETY: buf is filled only with ASCII hex characters from HEX_CHARS
        let s = unsafe { core::str::from_utf8_unchecked(&buf) };
        f.write_str(s)
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
