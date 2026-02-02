2024-05-24 - [Kernel Telemetry Hardening]
**Threat:** Undefined Behavior (UB) in `RingBuffer` due to casting immutable reference `&T` to mutable pointer `*mut T` and writing to it.
**Defense:** Replaced raw pointer casting with `UnsafeCell<T>` to provide sound interior mutability.

2024-05-24 - [Kernel Logger Race]
**Threat:** Potential data race and aliasing violation when accessing `static mut KERNEL_LOGGER`.
**Defense:** Used `core::ptr::addr_of!` to obtain raw pointer without creating intermediate references, avoiding reference aliasing rules violation.

2024-05-24 - [Test Stability]
**Threat:** SIGSEGV in tests due to privileged I/O instructions execution in userspace.
**Defense:** Gated unsafe hardware I/O blocks with `#[cfg(not(test))]`.
