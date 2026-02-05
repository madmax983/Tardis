## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## [Reduction]
**Bloat:** "Enterprise FizzBuzz" architecture in `tardis_common` (traits for single implementations) and mock structs in `chronos`.
**Cut:** Deleted `VortexService` and `GallifreyService` traits. Removed `MockVortex` and `MockGallifrey`. Added direct testability to `Vortex`.
**Saved:** Removed 2 traits, 2 mock structs, `traits.rs` file. Eliminated dynamic dispatch overhead and "mock maintenance".
