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

**[Redundant Allocations in Hot Paths]
**Learning:** Functions that preprocess inputs (e.g., `to_lowercase`) internally cause redundant allocations when called multiple times in a loop or pipeline.
**Action:** Hoist the preprocessing to the caller or the public API boundary and pass the prepared data (e.g., `&str` instead of `String`) to private helpers.
