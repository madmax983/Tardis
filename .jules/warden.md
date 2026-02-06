2024-05-24 - [Kernel Telemetry Hardening]
**Threat:** Undefined Behavior (UB) in `RingBuffer` due to casting immutable reference `&T` to mutable pointer `*mut T` and writing to it.
**Defense:** Replaced raw pointer casting with `UnsafeCell<T>` to provide sound interior mutability.

2024-05-24 - [Kernel Logger Race]
**Threat:** Potential data race and aliasing violation when accessing `static mut KERNEL_LOGGER`.
**Defense:** Used `core::ptr::addr_of!` to obtain raw pointer without creating intermediate references, avoiding reference aliasing rules violation.

2024-05-24 - [Test Stability]
**Threat:** SIGSEGV in tests due to privileged I/O instructions execution in userspace.
**Defense:** Gated unsafe hardware I/O blocks with `#[cfg(not(test))]`.

2025-02-23 - [RingBuffer Hardening]
**Threat:** Race condition in `RingBuffer::try_read` allowing torn reads, and potential buffer overread via corrupted `payload_len`.
**Defense:** Added Seqlock verification after read and capped `payload_len` to `MAX_PAYLOAD_SIZE`.

2025-02-23 - [Telemetry Allocation Removal]
**Threat:** Panic or deadlock in interrupt/no_std context due to allocation in `Subsystem::from_target`.
**Defense:** Replaced allocating case conversion with zero-allocation case-insensitive search.

2026-02-23 - [RingBuffer UB & Stuck Writer]
**Threat:** 1. `bytes` 1.11.0 Integer Overflow (DoS). 2. `RingBuffer` UB (Data Race in `copy_nonoverlapping` on shared memory). 3. `RingBuffer` Reader Livelock (infinite spin on stuck writer).
**Defense:** 1. Updated `bytes` to 1.11.1. 2. Replaced `copy_nonoverlapping` with `volatile` read/write loops. 3. Added retry limit and skip logic to `try_read`.

2026-03-01 - [RingBuffer Race Condition]
**Threat:** MPSC usage of SPSC RingBuffer allowed multiple producers to race on slot claiming, leading to concurrent writes to `UnsafeCell` (UB) and data corruption.
**Defense:** Implemented CAS (Compare-And-Swap) loop in `try_write` to enforce exclusive slot access, with spin-wait backoff for contention handling.
