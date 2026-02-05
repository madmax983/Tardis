# Atlas Journal

**[Refactor] Vortex Tokenizer De-Bloat**
**Tangle:** `vortex/src/tokenizer/mod.rs` was a "Blob" (747 lines) containing Service logic, Types, and Chat Template logic. It violated Single Responsibility Principle.
**Blueprint:** Extracted `types.rs` for data structures and `template.rs` for template logic.
**Stability:** Reduced `mod.rs` size, improved cohesion, and enforced strict visibility (`pub(crate) mod template`).

**[Architect] Decouple Chronos from Concrete Services**
**Tangle:** `Chronos` struct depended on concrete `Arc<Vortex>` and `Arc<Gallifrey>`, creating high coupling and making testing difficult.
**Blueprint:**
1.  Extracted domain types (`Entity`, `Message`, `Snapshot`, etc.) from `gallifrey` to `common/src/domain.rs`.
2.  Updated `GallifreyService` trait in `common` to include methods for conversation and system state.
3.  Refactored `Chronos` to use `Arc<dyn VortexService>` and `Arc<dyn GallifreyService>`.
**Stability:** `Chronos` is now decoupled from specific implementations, allowing for easier mocking and substitution.

**[Architect] Enforce Chronos Decoupling via Traits**
**Tangle:** `Chronos` was still coupled to concrete `Vortex` and `Gallifrey` structs despite previous plans. `Vortex` defined its own `ModelHandle`, causing type mismatches.
**Blueprint:**
1. Moved `ModelLoadConfig` and `InferenceParams` to `common::llm`.
2. Defined `VortexService` and `GallifreyService` in `common::traits`.
3. Refactored `vortex` to use `common::id::ModelHandle`.
4. Updated `Chronos` to use `Arc<dyn Service>`.
**Stability:** `Chronos` is now truly decoupled. `ModelHandle` is unified.

**[Refactor] Telemetry Types De-Blob**
**Tangle:** `telemetry/src/types.rs` was a "Blob" (649 lines) mixing Tracing, Logging, Metrics, and Routing concerns.
**Blueprint:**
1. Extracted `trace.rs` (IDs), `log.rs` (Level), `meta.rs` (Subsystem), `wire.rs` (TelemetryEntry), and `metrics_types.rs`.
2. Converted `types.rs` into a Facade re-exporting these modules.
**Stability:** Improved cohesion. `no_std` compatibility preserved.
