# Bard's Journal

## 2024-05-23 - Criterion Black Box Deprecation
**Confusion:** Benchmarks were failing clippy checks due to `criterion::black_box` usage.
**Clarification:** `criterion::black_box` is deprecated. Use `std::hint::black_box` (available since Rust 1.66) instead for modern Rust benchmarks.
