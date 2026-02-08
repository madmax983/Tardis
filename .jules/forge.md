**[Stable Toolchain Configuration]
**Learning:** Unstable rustfmt options and `rust_2024_compatibility` lint cause failures on stable toolchains.
**Action:** Ensure `rustfmt.toml` uses only stable options and `rust_2024_compatibility` is allowed in `Cargo.toml` for stable builds.

**[Refactoring Logic Chains]
**Learning:** Long `if-else if` chains for string matching are error-prone and hard to read.
**Action:** Replace with data-driven mapping arrays (e.g., `&[(&str, Enum)]`) where possible.

**[Hidden Time Dependencies]
**Learning:** Using `Utc::now()` deep inside business logic makes functions impure and untestable without mocking.
**Action:** Inject `now: DateTime<Utc>` as an argument to pure functions, passing the current time from the top level (e.g., controller or public API).

**[Metric Collection Performance]
**Learning:** Cumulative histograms on write (O(N) atomics) cause contention. Cumulative on read (O(N) read, O(1) write) is better.
**Action:** Prefer "write-fast, read-slow" for high-frequency telemetry data structures.

**[Safe Testing of Internal State]
**Learning:** Testing lock-free data structures often requires manipulating internal state (like read/write pointers) that should not be exposed.
**Action:** Use `#[cfg(test)]` to expose safe helper methods (e.g., `set_positions`) instead of using `unsafe` pointer arithmetic in tests.

**[Synchronous Initialization of Async Components]
**Learning:** `async` functions (like `OtlpExporter::new`) that don't await anything prevent usage in synchronous initialization paths (`init`).
**Action:** Remove `async` from constructors if they only perform synchronous setup (like spawning a background task), returning a `Result` immediately.
