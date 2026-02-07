## [Reduction]
**Bloat:** Speculative Generality and "One-Time Traits" (`VortexService`, `GallifreyService`, `ChronosService`).
**Cut:** Replaced traits with concrete Structs (`Vortex`, `Gallifrey`). Removed unused `ChronosService`.
**Saved:** 3 traits, 300+ lines of code, unnecessary casting, and cognitive load of following indirection.

## Unified QueryResult and Telemetry Simplification
**Bloat:** A `BatchProcessor` implementation that was never used by the OTLP exporter, a misplaced `QueryResult` struct in `common` pretending to be a trait, and a `TelemetryQuery` builder that was never instantiated.
**Cut:** Deleted `BatchProcessor`, `TelemetryQuery`, and `common/src/traits.rs`. Moved `QueryResult` to `gallifrey` and `TelemetryContext` to `store.rs`. Flattened `telemetry/src/export` and `gallifrey/src/query`.
**Saved:** ~200 lines of dead code and 3 unnecessary files/modules. Reduced cross-crate confusion about where `QueryResult` lives.
