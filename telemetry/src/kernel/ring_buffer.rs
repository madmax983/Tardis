//! Lock-free ring buffer for kernel telemetry.
//!
//! This module provides a Single-Producer Single-Consumer (SPSC) ring buffer
//! optimized for kernel telemetry:
//!
//! - Zero allocation after initialization
//! - Sub-microsecond write latency
//! - Safe for use in interrupt handlers
//! - Readable from userspace via shared memory
//!
//! # Design
//!
//! The ring buffer uses sequence numbers for lock-free synchronization:
//! - Each slot has a sequence number
//! - Odd sequence = slot being written
//! - Even sequence = slot ready to read
//!
//! # Memory Layout
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │ write_pos (8 bytes) │ read_pos (8 bytes) │ padding (48 bytes) │
//! ├────────────────────────────────────────────────────────────────┤
//! │ Slot 0 (cache-line aligned, 256 bytes)                        │
//! ├────────────────────────────────────────────────────────────────┤
//! │ Slot 1 ...                                                     │
//! ├────────────────────────────────────────────────────────────────┤
//! │ ... Slot N-1                                                   │
//! └────────────────────────────────────────────────────────────────┘
//! ```

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use crate::types::{TelemetryEntry, Level, Subsystem, EventType, TraceId, SpanId};

/// Ring buffer capacity (power of 2 for efficient modulo).
pub const RING_BUFFER_SIZE: usize = 4096;

/// Mask for efficient modulo operation.
const RING_BUFFER_MASK: usize = RING_BUFFER_SIZE - 1;

/// Maximum payload size per entry.
pub const MAX_PAYLOAD_SIZE: usize = 192;

/// Entry slot in the ring buffer.
///
/// Each slot is cache-line aligned to prevent false sharing between
/// producer and consumer.
#[repr(C, align(64))]
pub struct RingSlot {
    /// Sequence number for lock-free synchronization.
    /// - Odd = being written
    /// - Even = ready to read (value / 2 = number of times written)
    sequence: AtomicUsize,

    /// Telemetry entry header.
    entry: UnsafeCell<TelemetryEntry>,

    /// Inline payload storage.
    payload: UnsafeCell<[u8; MAX_PAYLOAD_SIZE]>,
}

impl RingSlot {
    /// Creates a new empty slot.
    const fn new() -> Self {
        Self {
            sequence: AtomicUsize::new(0),
            entry: UnsafeCell::new(TelemetryEntry {
                timestamp_ns: 0,
                level: Level::Trace,
                subsystem: Subsystem::Unknown,
                event_type: EventType::Unknown,
                span_id: SpanId::NONE,
                trace_id: TraceId::NONE,
                parent_span_id: SpanId::NONE,
                payload_len: 0,
            }),
            payload: UnsafeCell::new([0; MAX_PAYLOAD_SIZE]),
        }
    }
}

/// Lock-free SPSC ring buffer for telemetry.
///
/// This buffer is designed to be placed in a shared memory region
/// accessible by both kernel and userspace.
#[repr(C)]
pub struct RingBuffer {
    /// Write position (only producer advances).
    write_pos: AtomicUsize,

    /// Padding to put read_pos on a different cache line.
    _pad1: [u8; 56],

    /// Read position (only consumer advances).
    read_pos: AtomicUsize,

    /// Padding before slots.
    _pad2: [u8; 56],

    /// Dropped event counter.
    dropped_count: AtomicU64,

    /// Padding before slots.
    _pad3: [u8; 56],

    /// Ring buffer slots.
    slots: [RingSlot; RING_BUFFER_SIZE],
}

impl RingBuffer {
    /// Creates a new ring buffer.
    ///
    /// This should be called once during kernel initialization to set up
    /// the shared memory region.
    #[must_use]
    pub const fn new() -> Self {
        // Initialize all slots
        // Note: In const context, we can't use array::from_fn
        const EMPTY_SLOT: RingSlot = RingSlot::new();

        Self {
            write_pos: AtomicUsize::new(0),
            _pad1: [0; 56],
            read_pos: AtomicUsize::new(0),
            _pad2: [0; 56],
            dropped_count: AtomicU64::new(0),
            _pad3: [0; 56],
            slots: [EMPTY_SLOT; RING_BUFFER_SIZE],
        }
    }

    /// Writes an entry to the ring buffer.
    ///
    /// This is the producer-side operation, called from the kernel.
    ///
    /// # Returns
    ///
    /// Returns `true` if the entry was written, `false` if the buffer is full.
    /// When the buffer is full, the oldest unread entry is overwritten.
    ///
    /// # Safety
    ///
    /// This method is safe to call from interrupt handlers as it uses
    /// only atomic operations and never blocks.
    #[inline]
    pub fn try_write(&self, entry: &TelemetryEntry, payload: &[u8]) -> bool {
        // Get current write position
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed);
        let index = pos & RING_BUFFER_MASK;

        // Get the slot
        let slot = &self.slots[index];

        // Check if slot is available (not being read)
        // We use the sequence number to track this
        let expected_seq = pos.wrapping_mul(2);
        let current_seq = slot.sequence.load(Ordering::Acquire);

        // If the slot is too far behind, we're overwriting unread data
        if current_seq != expected_seq && current_seq != 0 {
            self.dropped_count.fetch_add(1, Ordering::Relaxed);
        }

        // Mark slot as being written (odd sequence)
        slot.sequence.store(expected_seq | 1, Ordering::Release);

        // Write the entry
        // SAFETY: We have exclusive access to this slot via the sequence protocol
        unsafe {
            let entry_ptr = slot.entry.get();
            core::ptr::write_volatile(entry_ptr, entry.clone());

            // Write payload
            let payload_len = payload.len().min(MAX_PAYLOAD_SIZE);
            let payload_ptr = slot.payload.get() as *mut u8;
            core::ptr::copy_nonoverlapping(payload.as_ptr(), payload_ptr, payload_len);

            // Update payload length
            (*entry_ptr).payload_len = payload_len as u16;
        }

        // Mark slot as ready to read (even sequence, incremented)
        slot.sequence.store(expected_seq.wrapping_add(2), Ordering::Release);

        true
    }

    /// Reads an entry from the ring buffer.
    ///
    /// This is the consumer-side operation, called from userspace.
    ///
    /// # Returns
    ///
    /// Returns `Some((entry, payload))` if an entry is available,
    /// `None` if the buffer is empty.
    pub fn try_read(&self) -> Option<(TelemetryEntry, alloc::vec::Vec<u8>)> {
        loop {
            let read_pos = self.read_pos.load(Ordering::Relaxed);
            let write_pos = self.write_pos.load(Ordering::Acquire);

            // Check if buffer is empty
            if read_pos >= write_pos {
                return None;
            }

            let index = read_pos & RING_BUFFER_MASK;
            let slot = &self.slots[index];

            // Check if slot is ready to read (even sequence)
            let expected_seq = read_pos.wrapping_mul(2).wrapping_add(2);
            let current_seq = slot.sequence.load(Ordering::Acquire);

            if current_seq != expected_seq {
                // Slot not ready yet, spin or return
                // In practice, this rarely happens
                core::hint::spin_loop();
                continue;
            }

            // Try to claim this slot
            if self
                .read_pos
                .compare_exchange(read_pos, read_pos + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
            {
                // Another reader got it, retry
                continue;
            }

            // Read the entry
            // SAFETY: We have exclusive read access via compare_exchange
            let entry = unsafe {
                let entry_ptr = slot.entry.get();
                core::ptr::read_volatile(entry_ptr)
            };

            let payload_len = entry.payload_len as usize;
            let mut payload = alloc::vec![0u8; payload_len];

            unsafe {
                core::ptr::copy_nonoverlapping(
                    slot.payload.get() as *const u8,
                    payload.as_mut_ptr(),
                    payload_len,
                );
            }

            return Some((entry, payload));
        }
    }

    /// Returns the number of entries available to read.
    #[must_use]
    pub fn available(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        write_pos.saturating_sub(read_pos)
    }

    /// Returns the number of dropped entries.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }

    /// Checks if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    /// Checks if the buffer is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.available() >= RING_BUFFER_SIZE
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: RingBuffer uses only atomic operations for synchronization
unsafe impl Sync for RingBuffer {}
unsafe impl Send for RingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_write_read() {
        // Use static to avoid stack overflow (RingBuffer is ~1MB)
        static BUFFER: RingBuffer = RingBuffer::new();

        let entry = TelemetryEntry {
            timestamp_ns: 12345,
            level: Level::Info,
            subsystem: Subsystem::Kernel,
            event_type: EventType::Boot,
            span_id: SpanId::NONE,
            trace_id: TraceId::NONE,
            parent_span_id: SpanId::NONE,
            payload_len: 0,
        };

        assert!(BUFFER.try_write(&entry, b"test payload"));
        assert_eq!(BUFFER.available(), 1);

        let (read_entry, payload) = BUFFER.try_read().unwrap();
        assert_eq!(read_entry.timestamp_ns, 12345);
        assert_eq!(read_entry.level, Level::Info);
        assert_eq!(&payload, b"test payload");
        assert!(BUFFER.is_empty());
    }

    #[test]
    fn ring_buffer_empty() {
        // Use static to avoid stack overflow (RingBuffer is ~1MB)
        static BUFFER: RingBuffer = RingBuffer::new();
        assert!(BUFFER.is_empty());
        assert!(BUFFER.try_read().is_none());
    }
}
