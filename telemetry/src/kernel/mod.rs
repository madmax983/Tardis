//! Kernel telemetry module.
//!
//! Provides `no_std` compatible telemetry for the Tardis kernel, enabling
//! high-speed logging and tracing from ring 0.
//!
//! # Components
//!
//! -   [`RingBuffer`]: The primary transport mechanism. A lock-free, SPSC shared memory
//!     buffer that connects the kernel (producer) to userspace (consumer).
//! -   [`KernelLogger`]: Implements the standard `log::Log` trait, routing `log::info!`,
//!     `log::error!`, etc., to the ring buffer.
//! -   [`serial`]: A direct hardware interface to the COM1 serial port, used as a
//!     fail-safe for critical errors or when the ring buffer is unavailable.
//!
//! # Architecture
//!
//! The system is designed to minimize kernel-side latency. Complex processing (formatting,
//! storage, network export) is offloaded to userspace.
//!
//! ```text
//!                                    Shared Memory
//! ┌────────────────┐              ┌────────────────┐              ┌───────────────┐
//! │     Kernel     │              │   Ring Buffer  │              │   Userspace   │
//! │                │              │                │              │               │
//! │  [log::info!]──┼─────────────►│ [Entry 1.....] │─────────────►│ [Drainer]     │
//! │                │              │ [Entry 2.....] │              │    │          │
//! │  [Panic!]──────┼─────────┐    │ [..........]   │              │    ▼          │
//! └────────────────┘         │    └────────────────┘              │ [Gallifrey]   │
//!                            │                                    └───────────────┘
//!                            │
//!                            ▼
//!                       ┌──────────┐
//!                       │ Serial   │
//!                       │ (COM1)   │
//!                       └──────────┘
//! ```

#![allow(unsafe_code)]

mod logger;
mod ring_buffer;
pub mod serial;

pub use logger::KernelLogger;
pub use ring_buffer::{RingBuffer, RingSlot, RING_BUFFER_SIZE};
