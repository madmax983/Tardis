## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** "Enterprise FizzBuzz" in `chronos::analyzer`. Generic closures + optimizations for <30 char strings + `TemporalRule` struct for 3 keywords.
**Cut:** Removed generics, removed length-based optimization, used direct string matching.
**Saved:** ~50 lines of complex code, removed `TemporalRule` and `TimeOffset` types, removed `contains_ignore_ascii_case` helper.

## [Reduction]
**Bloat:** Over-engineered "PsychicPaper" interpreter with `Intent` enum dispatch, stateful struct, and heuristic guessing for specific formats.
**Cut:** Replaced with stateless module functions (`extract_json`, `parse_list`, `parse_kv`, `repair_structure`). Removed `PsychicPaper` struct and `Intent` enum.
**Saved:** ~50 lines of boilerplate, removed enum dispatch, clearer call sites, fixed buggy behavior in `SonicScrewdriver`.
