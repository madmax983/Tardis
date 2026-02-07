//! Kernel logger implementation.
//!
//! Implements `log::Log` to capture all kernel log messages and write them
//! to the telemetry ring buffer.

use crate::kernel::ring_buffer::RingBuffer;
use crate::kernel::serial;
use crate::types::{EventType, Level, SpanId, Subsystem, TelemetryEntry, TraceId};
use core::sync::atomic::{AtomicU8, Ordering};

/// Global kernel telemetry state.
static mut KERNEL_LOGGER: Option<KernelLogger> = None;

/// Logger initialization state.
static INITIALIZED: AtomicU8 = AtomicU8::new(STATE_UNINIT);

const STATE_UNINIT: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_INITIALIZED: u8 = 2;

#[cfg(test)]
pub fn reset_for_test() {
    INITIALIZED.store(STATE_UNINIT, Ordering::SeqCst);
    unsafe {
        KERNEL_LOGGER = None;
    }
}

#[cfg(test)]
pub fn is_initialized_for_test() -> bool {
    INITIALIZED.load(Ordering::Relaxed) == STATE_INITIALIZED
}

/// Kernel logger that writes to the ring buffer.
#[allow(missing_debug_implementations)]
pub struct KernelLogger {
    /// Ring buffer for userspace consumption.
    ring_buffer: &'static RingBuffer,

    /// Whether serial output is enabled.
    serial_enabled: bool,

    /// Current trace context.
    current_trace: TraceId,

    /// Current span context.
    current_span: SpanId,
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
            current_trace: TraceId::NONE,
            current_span: SpanId::NONE,
        }
    }

    /// Initializes the global kernel logger.
    ///
    /// # Safety
    ///
    /// This function must be called exactly once during kernel initialization,
    /// before any logging occurs.
    ///
    /// # Panics
    ///
    /// Panics if called more than once.
    #[allow(clippy::panic, clippy::manual_assert)]
    pub unsafe fn init(ring_buffer: &'static RingBuffer, serial_enabled: bool) {
        match INITIALIZED.compare_exchange(
            STATE_UNINIT,
            STATE_INITIALIZING,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // We won the race to initialize
            }
            Err(STATE_INITIALIZING) => {
                panic!("KernelLogger::init called concurrently");
            }
            Err(STATE_INITIALIZED) => {
                panic!("KernelLogger::init called more than once");
            }
            Err(_) => unreachable!(),
        }

        if serial_enabled {
            serial::init();
        }

        unsafe {
            KERNEL_LOGGER = Some(KernelLogger::new(ring_buffer, serial_enabled));
        }

        // Mark as fully initialized.
        // Use Release ordering to ensure KERNEL_LOGGER write is visible before this.
        INITIALIZED.store(STATE_INITIALIZED, Ordering::Release);
    }

    /// Gets a reference to the global logger.
    ///
    /// Returns `None` if the logger hasn't been initialized.
    #[must_use]
    pub fn get() -> Option<&'static Self> {
        if INITIALIZED.load(Ordering::Acquire) == STATE_INITIALIZED {
            // SAFETY: We only set KERNEL_LOGGER once during init
            unsafe { (*core::ptr::addr_of!(KERNEL_LOGGER)).as_ref() }
        } else {
            None
        }
    }

    /// Logs a message.
    pub fn log(&self, level: Level, subsystem: Subsystem, message: &str) {
        let entry = TelemetryEntry {
            timestamp_ns: self.read_timestamp(),
            level,
            subsystem,
            event_type: EventType::Log,
            span_id: self.current_span,
            trace_id: self.current_trace,
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
            span_id: self.current_span,
            trace_id: self.current_trace,
            parent_span_id: SpanId::NONE,
            payload_len: 0,
        };

        self.ring_buffer.try_write(&entry, payload);
    }

    /// Sets the current trace context.
    pub const fn set_trace_context(&mut self, trace_id: TraceId, span_id: SpanId) {
        self.current_trace = trace_id;
        self.current_span = span_id;
    }

    /// Clears the current trace context.
    pub const fn clear_trace_context(&mut self) {
        self.current_trace = TraceId::NONE;
        self.current_span = SpanId::NONE;
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
        serial::write_str("[");
        serial::write_str(level.as_str());
        serial::write_str("] ");
        serial::write_str(subsystem.as_str());
        serial::write_str(": ");
        serial::write_str(message);
        serial::write_str("\n");
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
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_race_condition_init() {
        super::reset_for_test();

        // Use static to avoid stack overflow (RingBuffer is ~1MB)
        static RING_BUFFER: RingBuffer = RingBuffer::new();

        // Spawn init thread
        let handle = thread::Builder::new()
            .name("init_thread".into())
            .spawn(move || unsafe {
                KernelLogger::init(&RING_BUFFER, false);
            })
            .unwrap();

        // Check for race condition
        // We expect that at some point get() returns None AND initialized is true
        // Because of the 10ms delay inserted in init(), this window is huge.
        let start = std::time::Instant::now();
        let mut race_detected = false;

        while start.elapsed() < Duration::from_millis(100) {
            let initialized = super::is_initialized_for_test();
            let logger = KernelLogger::get();

            if initialized && logger.is_none() {
                race_detected = true;
                break;
            }

            if logger.is_some() {
                break; // Initialized successfully
            }

            thread::yield_now();
        }

        handle.join().unwrap();

        if race_detected {
            panic!("Race condition detected: INITIALIZED is true but get() returned None");
        }
    }
}
