//! Ring buffer drainer for kernel telemetry.
//!
//! This module reads telemetry events from the kernel ring buffer
//! and converts them to tracing spans for integration with the
//! userspace telemetry system.
//!
//! Note: The actual ring buffer integration requires shared memory
//! between kernel and userspace, which is not yet implemented.

use crate::types::{Level, TelemetryEntry};
use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration for the ring buffer drainer.
#[derive(Debug, Clone)]
pub struct DrainerConfig {
    /// How often to poll the ring buffer.
    #[allow(dead_code)] // Will be used when ring buffer integration is complete
    pub poll_interval: Duration,

    /// Maximum entries to process per poll.
    #[allow(dead_code)] // Will be used when ring buffer integration is complete
    pub batch_size: usize,
}

impl Default for DrainerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(10),
            batch_size: 100,
        }
    }
}

/// Placeholder drainer for when kernel integration is available.
///
/// This drainer will eventually read from a shared memory ring buffer
/// populated by the kernel, converting kernel events to tracing spans.
#[derive(Debug)]
pub struct RingBufferDrainer {
    /// Configuration.
    #[allow(dead_code)] // Will be used when ring buffer integration is complete
    config: DrainerConfig,

    /// Channel for receiving entries (for testing/simulation).
    #[allow(dead_code)] // Will be used when ring buffer integration is complete
    receiver: Option<mpsc::Receiver<(TelemetryEntry, Vec<u8>)>>,
}

#[allow(dead_code)] // Placeholder structure not yet fully integrated
impl RingBufferDrainer {
    /// Creates a new drainer with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: DrainerConfig::default(),
            receiver: None,
        }
    }

    /// Sets the configuration.
    #[must_use]
    pub fn with_config(mut self, config: DrainerConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets a receiver for simulated kernel events.
    #[must_use]
    pub fn with_receiver(mut self, receiver: mpsc::Receiver<(TelemetryEntry, Vec<u8>)>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Runs the drainer loop.
    ///
    /// This will process events from the receiver if configured.
    pub async fn run(mut self) {
        let Some(mut receiver) = self.receiver.take() else {
            tracing::warn!(
                "RingBufferDrainer started without receiver - no kernel events will be processed"
            );
            return;
        };

        while let Some((entry, payload)) = receiver.recv().await {
            self.process_entry(&entry, &payload);
        }
    }

    /// Processes a single telemetry entry.
    fn process_entry(&self, entry: &TelemetryEntry, payload: &[u8]) {
        let message = std::str::from_utf8(payload).unwrap_or("<binary>");

        match entry.level {
            Level::Trace => tracing::trace!(
                subsystem = %entry.subsystem,
                event_type = %entry.event_type,
                timestamp_ns = entry.timestamp_ns,
                "[kernel] {}", message
            ),
            Level::Debug => tracing::debug!(
                subsystem = %entry.subsystem,
                event_type = %entry.event_type,
                timestamp_ns = entry.timestamp_ns,
                "[kernel] {}", message
            ),
            Level::Info => tracing::info!(
                subsystem = %entry.subsystem,
                event_type = %entry.event_type,
                timestamp_ns = entry.timestamp_ns,
                "[kernel] {}", message
            ),
            Level::Warn => tracing::warn!(
                subsystem = %entry.subsystem,
                event_type = %entry.event_type,
                timestamp_ns = entry.timestamp_ns,
                "[kernel] {}", message
            ),
            Level::Error => tracing::error!(
                subsystem = %entry.subsystem,
                event_type = %entry.event_type,
                timestamp_ns = entry.timestamp_ns,
                "[kernel] {}", message
            ),
        }
    }
}

impl Default for RingBufferDrainer {
    fn default() -> Self {
        Self::new()
    }
}
