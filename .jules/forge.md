**[God Function Detected]
**Learning:** `Vortex::load_model` was accumulating too many responsibilities: I/O, parsing, concurrent weight loading, registry management, and tokenizer setup.
**Action:** Extracted `load_weights_task`, `register_loaded_model`, and `setup_tokenizer` to keep the main flow readable. Future additions to model loading should follow this pattern of small, dedicated helpers.

**[CI & Dependency Hell]
**Learning:** `cargo test --all-features` is dangerous in a workspace with platform-specific features (like `metal` on macOS) because it enables them on all platforms, causing build failures on Linux/Windows. Also, `cargo-deny` v2 requires config updates.
**Action:** Use conditional CI steps for platform-specific tests instead of blanket `--all-features`. Update `deny.toml` when upgrading actions. Check MSRV compatibility of dependencies (like `home` requiring rustc 1.88) and pin if necessary.
