## [Reduction]
**Bloat:** Speculative "Service" traits (`VortexService`, `GallifreyService`, `ChronosService`) in `common` crate that were only implemented by their respective concrete structs and never used polymorphically.
**Cut:** Deleted `common/src/traits.rs` and removed the `impl VortexService for Vortex` block. Updated `chronos` and `shell` to remove unused `async` and unnecessary state (`CommandHandler` instance).
**Saved:** ~300 lines of code. Eliminated "Generic Soup" and "Layer Lasagna". Direct calls are now used, improving clarity and build times.
