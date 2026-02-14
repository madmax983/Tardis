## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** Deep folder hierarchies for small modules (`chronos/memory`, `vortex/model`, `gallifrey/domain`, `shell/experimental`).
**Cut:** Flattened 4 directories into single files (`memory.rs`, `model.rs`, `domain.rs`, `experimental.rs`).
**Saved:** 4 directories, 4 `mod.rs` files, and reduced file navigation overhead.
