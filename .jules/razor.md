## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** "Enterprise FizzBuzz" in `chronos::analyzer`. Generic closures + optimizations for <30 char strings + `TemporalRule` struct for 3 keywords.
**Cut:** Removed generics, removed length-based optimization, used direct string matching.
**Saved:** ~50 lines of complex code, removed `TemporalRule` and `TimeOffset` types, removed `contains_ignore_ascii_case` helper.

## [Reduction]
**Bloat:** Speculative Features ("Prophet", "Fugue", "Dashboard") that used hallucinations to predict future system states.
**Cut:** Deleted `prophecy.rs`, `fugue.rs`, `dashboard.rs` and related commands.
**Saved:** 3 files, ~500 lines of speculative code, removed dependency on unpredictable LLM outputs for core UI.

## [Reduction]
**Bloat:** "Factory Factory" / Manager pattern in `Vortex`. `ModelRegistry` was a separate struct just to hold a map, duplicating state in `Vortex`.
**Cut:** Merged `ModelRegistry` into `Vortex`. Flattened `vortex/src/model/` directory.
**Saved:** 1 file, removed `ModelRegistry` struct, removed `RwLock` indirection, simplified model loading logic.

## [Reduction]
**Bloat:** "Enterprise FizzBuzz" in `chronos::analyzer`. `IntentRule` struct and array for simple keyword matching.
**Cut:** Replaced with inline `if/else` checks.
**Saved:** ~30 lines of boilerplate, removed `IntentRule` struct.
