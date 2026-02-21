//! Kernel logger implementation.
//!
//! Implements `log::Log` to capture all kernel log messages and write them
//! to the telemetry ring buffer.

use crate::kernel::ring_buffer::RingBuffer;
use crate::kernel::serial;
use crate::types::{EventType, Level, SpanId, Subsystem, TelemetryEntry, TraceId};
use spin::Once;

/// Global kernel telemetry state.
static KERNEL_LOGGER: Once<KernelLogger> = Once::new();

/// Kernel logger that writes to the ring buffer.
#[allow(missing_debug_implementations)]
pub struct KernelLogger {
    /// Ring buffer for userspace consumption.
    ring_buffer: &'static RingBuffer,

    /// Whether serial output is enabled.
    serial_enabled: bool,
}

impl KernelLogger {
    /// Creates a new kernel logger.
    ///
    /// # Arguments
    ///
    /// * `ring_buffer` - Static reference to the shared ring buffer
    /// * `serial_enabled` - Whether to also output to serial port
    #[must_use]
    pub const fn new(ring_buffer: &'static RingBuffer, serial_enabled: bool) -> Self {
        Self {
            ring_buffer,
            serial_enabled,
        }
    }

    /// Initializes the global kernel logger.
    ///
    /// This function is idempotent. It ensures the logger is initialized exactly once.
    /// Subsequent calls will be ignored.
    #[allow(clippy::panic, clippy::manual_assert)]
    pub fn init(ring_buffer: &'static RingBuffer, serial_enabled: bool) {
        KERNEL_LOGGER.call_once(|| {
            if serial_enabled {
                // SAFETY: We are in kernel initialization phase.
                unsafe { serial::init() };
            }
            KernelLogger::new(ring_buffer, serial_enabled)
        });

        // Note: In actual kernel, we would call log::set_logger here
        // For now, we provide manual logging functions
    }

    /// Gets a reference to the global logger.
    ///
    /// Returns `None` if the logger hasn't been initialized.
    #[must_use]
    pub fn get() -> Option<&'static Self> {
        KERNEL_LOGGER.get()
    }

    /// Logs a message.
    pub fn log(&self, level: Level, subsystem: Subsystem, message: &str) {
        let entry = TelemetryEntry {
            timestamp_ns: self.read_timestamp(),
            level,
            subsystem,
            event_type: EventType::Log,
            span_id: SpanId::NONE,
            trace_id: TraceId::NONE,
            parent_span_id: SpanId::NONE,
            payload_len: 0, // Set by ring buffer
        };

        let payload = message.as_bytes();

        // Try to write to ring buffer
        if !self.ring_buffer.try_write(&entry, payload) {
            // Buffer full - write to serial as fallback
            if self.serial_enabled {
                self.write_serial(level, subsystem, message);
            }
        } else if self.serial_enabled && level >= Level::Warn {
            // Also write warnings and errors to serial
            self.write_serial(level, subsystem, message);
        }
    }

    /// Logs an event with a specific event type.
    pub fn log_event(
        &self,
        level: Level,
        subsystem: Subsystem,
        event_type: EventType,
        payload: &[u8],
    ) {
        let entry = TelemetryEntry {
            timestamp_ns: self.read_timestamp(),
            level,
            subsystem,
            event_type,
            span_id: SpanId::NONE,
            trace_id: TraceId::NONE,
            parent_span_id: SpanId::NONE,
            payload_len: 0,
        };

        self.ring_buffer.try_write(&entry, payload);
    }

    /// Reads the current timestamp in nanoseconds.
    ///
    /// In kernel context, this would use TSC or HPET.
    /// For now, returns a placeholder.
    #[allow(clippy::unused_self)]
    fn read_timestamp(&self) -> u64 {
        // TODO: Use actual TSC or HPET in kernel
        // This is a placeholder that increments
        static COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
        COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
    }

    /// Writes a message to the serial port.
    #[allow(clippy::unused_self)]
    fn write_serial(&self, level: Level, subsystem: Subsystem, message: &str) {
        // Format: [LEVEL] subsystem: message\n
        // SAFETY: We assume we are in kernel context if KernelLogger is active with serial_enabled=true.
        // This relies on the invariant established by `init`.
        unsafe {
            serial::write_str("[");
            serial::write_str(level.as_str());
            serial::write_str("] ");
            serial::write_str(subsystem.as_str());
            serial::write_str(": ");
            serial::write_str(message);
            serial::write_str("\n");
        }
    }
}

/// Convenience macro for kernel logging.
#[macro_export]
macro_rules! klog {
    ($level:expr, $subsystem:expr, $($arg:tt)*) => {{
        if let Some(logger) = $crate::kernel::KernelLogger::get() {
            use ::alloc::format;
            let message = format!($($arg)*);
            logger.log($level, $subsystem, &message);
        }
    }};
}

/// Log an info message from the kernel.
#[macro_export]
macro_rules! kinfo {
    ($subsystem:expr, $($arg:tt)*) => {
        $crate::klog!($crate::Level::Info, $subsystem, $($arg)*)
    };
}

/// Log an error message from the kernel.
#[macro_export]
macro_rules! kerror {
    ($subsystem:expr, $($arg:tt)*) => {
        $crate::klog!($crate::Level::Error, $subsystem, $($arg)*)
    };
}

/// Log a warning message from the kernel.
#[macro_export]
macro_rules! kwarn {
    ($subsystem:expr, $($arg:tt)*) => {
        $crate::klog!($crate::Level::Warn, $subsystem, $($arg)*)
    };
}

/// Log a debug message from the kernel.
#[macro_export]
macro_rules! kdebug {
    ($subsystem:expr, $($arg:tt)*) => {
        $crate::klog!($crate::Level::Debug, $subsystem, $($arg)*)
    };
}

/// Log a trace message from the kernel.
#[macro_export]
macro_rules! ktrace {
    ($subsystem:expr, $($arg:tt)*) => {
        $crate::klog!($crate::Level::Trace, $subsystem, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_initialization() {
        // Reset state for testing (unsafe but necessary for test isolation if possible)
        // Since we can't easily reset static mut safely in parallel tests,
        // we assume this test runs in isolation or first.
        // However, we can't guarantee that.
        // So we will just test the state transitions we can observe.

        // If it's already initialized by another test, we can't test pre-init state.
        // But in a fresh test run, it should be uninitialized.

        static RING_BUFFER: RingBuffer = RingBuffer::new();

        // Check if already initialized (by another test?)
        if KernelLogger::get().is_some() {
            // Already initialized, can't test transition.
            return;
        }

        assert!(KernelLogger::get().is_none());

        KernelLogger::init(&RING_BUFFER, false);

        assert!(KernelLogger::get().is_some());
    }
}
