## [Reduction]
**Bloat:** The "One-Time" Trait and Layer Lasagna. `VortexService`, `ChronosService`, `GallifreyService` traits were defined but only `VortexService` was implemented (and only once). It introduced duplicate types and needless mapping logic.
**Cut:** Deleted `common/src/traits.rs` and removed the `impl` in `vortex/src/inference/runtime.rs`. Exposed `Vortex` struct directly.
**Saved:** ~260 lines of code. Reduced type duplication and improved locality of behavior.
