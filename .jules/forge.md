**[Stable Toolchain Configuration]
**Learning:** Unstable rustfmt options and `rust_2024_compatibility` lint cause failures on stable toolchains.
**Action:** Ensure `rustfmt.toml` uses only stable options and `rust_2024_compatibility` is allowed in `Cargo.toml` for stable builds.

**[Refactoring Logic Chains]
**Learning:** Long `if-else if` chains for string matching are error-prone and hard to read.
**Action:** Replace with data-driven mapping arrays (e.g., `&[(&str, Enum)]`) where possible.
