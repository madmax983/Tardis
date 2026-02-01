# Bolt's Journal

## [Clippy Noise]
**Learning:** Running `cargo clippy --all-targets --all-features` can fail if unrelated dependencies have build errors (e.g., missing system libs).
**Action:** Use `cargo clippy -p <crate_name> --all-features` to isolate checks to the modified crate when broad checks fail.
