## 2024-05-23 - [Build System]
**Confusion:** `cargo clippy --all-features` fails on Linux due to `objc_exception`.
**Clarification:** Some workspace crates pull in macOS-specific dependencies. Run clippy per-crate or without `--all-features` to avoid this.
