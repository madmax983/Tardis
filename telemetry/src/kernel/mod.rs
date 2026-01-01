//! Kernel telemetry module.
//!
//! Provides no_std compatible telemetry for the Tardis kernel:
//!
//! - [`RingBuffer`]: Lock-free SPSC ring buffer for event storage
//! - [`KernelLogger`]: `log::Log` implementation writing to ring buffer
//! - [`serial`]: Serial port output for early boot and panic messages
//!
//! # Architecture
//!
//! The kernel writes telemetry entries to a ring buffer that is readable
//! from userspace. This allows the kernel to emit events with minimal
//! overhead while userspace handles storage and export.
//!
//! ```text
//! Kernel (no_std)              Userspace (std)
//! ┌─────────────┐              ┌─────────────┐
//! │ log::info!()│──┐           │   Drainer   │
//! │ log::error!()│  │           │             │
//! └─────────────┘  │           └──────▲──────┘
//!                  │                  │
//!                  ▼                  │
//!            ┌───────────────────────────┐
//!            │       Ring Buffer         │
//!            │  (shared memory region)   │
//!            └───────────────────────────┘
//! ```

mod ring_buffer;
mod logger;
pub mod serial;

pub use ring_buffer::{RingBuffer, RingSlot, RING_BUFFER_SIZE};
pub use logger::KernelLogger;
