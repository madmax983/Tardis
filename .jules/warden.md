# Warden's Journal

## 2025-05-24 - Hardened Kernel Telemetry

**Threat:** `static mut` usage in `KernelLogger` created a risk of undefined behavior if mutable references were created concurrently, or if trace context was updated without synchronization.
**Defense:** Refactored `KernelLogger` to be effectively immutable after initialization. Removed mutable fields (`current_trace`, `current_span`) and methods (`set_trace_context`, `clear_trace_context`). Added safety documentation explaining the safety of the `static mut` in this restricted context.

**Threat:** `unsafe` blocks in `RingBuffer` used `volatile` reads/writes without explicit documentation, raising questions about correctness and necessity.
**Defense:** Added detailed comments to `try_write` and `try_read` explaining that `volatile` operations are required to prevent the compiler from optimizing away memory accesses in the lock-free Seqlock pattern, effectively treating the shared memory as I/O mapped memory to avoid UB from data races on the payload buffer.

**Threat:** `unsafe` port I/O in `serial.rs`.
**Defense:** Added safety comments explaining that port I/O is restricted to kernel features and guarded by initialization checks.
