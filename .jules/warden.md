# Warden's Journal

## 2025-05-24 - Hardened Kernel Telemetry

**Threat:** `static mut` usage in `KernelLogger` created a risk of undefined behavior if mutable references were created concurrently, or if trace context was updated without synchronization.
**Defense:** Refactored `KernelLogger` to be effectively immutable after initialization. Removed mutable fields (`current_trace`, `current_span`) and methods (`set_trace_context`, `clear_trace_context`). Added safety documentation explaining the safety of the `static mut` in this restricted context.

**Threat:** `unsafe` blocks in `RingBuffer` used `volatile` reads/writes without explicit documentation, raising questions about correctness and necessity.
**Defense:** Added detailed comments to `try_write` and `try_read` explaining that `volatile` operations are required to prevent the compiler from optimizing away memory accesses in the lock-free Seqlock pattern, effectively treating the shared memory as I/O mapped memory to avoid UB from data races on the payload buffer.

**Threat:** `unsafe` port I/O in `serial.rs`.
**Defense:** Added safety comments explaining that port I/O is restricted to kernel features and guarded by initialization checks.

**2025-06-01 - RingBuffer Enum Safety**
**Threat:** `RingBuffer` used `ptr::read_volatile` to read `TelemetryEntry` structs containing Rust enums (`Level`, `Subsystem`, `EventType`) directly from potentially corrupted shared memory. Reading an invalid discriminant (e.g., `5` for `Level`) is immediate Undefined Behavior.
**Defense:** Introduced `TelemetryEntryRaw` with integer fields to mirror the memory layout. Modified `RingBuffer` to read raw integers first, then safely convert them to valid enum variants (mapping invalid values to `Info`/`Unknown`), eliminating the UB risk.

**2025-06-02 - KernelLogger Race Condition**
**Threat:** A race condition in `KernelLogger::init` allowed `get()` to access `KERNEL_LOGGER` before it was fully initialized, leading to Undefined Behavior (reading partially written `static mut`).
**Defense:** Replaced the boolean initialization flag with an `AtomicU8` state machine (`UNINIT`, `INITING`, `INITED`). Used `Release` ordering in `init` and `Acquire` ordering in `get` to enforce a happens-before relationship, ensuring `KERNEL_LOGGER` is only accessed after initialization is complete and visible.

**2025-06-03 - RingBuffer Mixed Volatile Access**
**Threat:** `RingBuffer::try_write` updated the `payload_len` field using a non-volatile write (`(*ptr).payload_len = ...`) after writing the struct header volatilely. This mixed volatile and non-volatile access to the same memory location, creating potential Undefined Behavior (data race) if a reader accessed it concurrently, as the compiler could reorder or tear the non-volatile write despite Seqlock protection.
**Defense:** Refactored `try_write` to update the `payload_len` in the local `TelemetryEntryRaw` copy *before* the volatile write. Now uses a single `ptr::write_volatile` to write the entire header, ensuring all shared memory writes are volatile and atomic with respect to compiler optimizations.

**2025-06-04 - Unsafe I/O and Dead Code**
**Threat:** Publicly exposed functions in `telemetry::kernel::serial` (`init`, `write_byte`, etc.) performed raw I/O port access while being marked safe. Userspace applications enabling the `kernel` feature could inadvertently call these functions, causing segmentation faults or UB.
**Defense:** Marked all `serial` module functions as `unsafe` and added comprehensive safety documentation requiring kernel context (Ring 0). Updated `KernelLogger` to wrap these calls in `unsafe` blocks.

**Threat:** Unused `BumpAllocator` in `kernel/src/memory/heap.rs` contained Undefined Behavior (casting `&self` to `*mut Self` to mutate fields) and was dead code.
**Defense:** Deleted `kernel/src/memory/heap.rs` and removed the module declaration, adhering to YAGNI and eliminating the safety risk.

**2025-06-05 - Supply Chain Attack**
**Threat:** `serde_json` v1.0.149 introduced a malicious dependency `zmij` v1.0.19. Additionally, `zip` v7.4.0 (a typosquatted/hijacked crate version) was pulled in by `candle-core` v0.9.2. These compromised versions posed a critical risk of arbitrary code execution.
**Defense:** Pinned `serde_json` to known safe version `=1.0.128`. Downgraded `candle-core` to `0.9.1` and patched the dependency to use the official Git repository, forcing dependency resolution to safe versions (e.g., `zip` v1.1.4). Removed compromised crate versions from `Cargo.lock`.
