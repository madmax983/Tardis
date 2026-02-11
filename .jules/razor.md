## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** Speculative Generality and Dead Code in `tardis-chronos` (`experimental` module, `memory` module).
**Cut:** Removed `Dreamer`, `Echoes`, `Historian`, `Prophet`, `PsychicPaper`, `SystemDoctor`, and `MemoryConsolidator`. Refactored `tardis-shell` to remove `Prophet` usage.
**Saved:** 7 unused structs/modules, ~1000 lines of code, 1 external dependency (`uuid` in chronos), and eliminated "feature flags for features that don't exist".
