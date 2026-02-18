## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** "Enterprise FizzBuzz" in `chronos::analyzer`. Generic closures + optimizations for <30 char strings + `TemporalRule` struct for 3 keywords.
**Cut:** Removed generics, removed length-based optimization, used direct string matching.
**Saved:** ~50 lines of complex code, removed `TemporalRule` and `TimeOffset` types, removed `contains_ignore_ascii_case` helper.
