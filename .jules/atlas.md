# Atlas Journal

**[Refactor] Vortex Tokenizer De-Bloat**
**Tangle:** `vortex/src/tokenizer/mod.rs` was a "Blob" (747 lines) containing Service logic, Types, and Chat Template logic. It violated Single Responsibility Principle.
**Blueprint:** Extracted `types.rs` for data structures and `template.rs` for template logic.
**Stability:** Reduced `mod.rs` size, improved cohesion, and enforced strict visibility (`pub(crate) mod template`).

**[Architect] Decouple Chronos from Concrete Services**
**Tangle:** `Chronos` struct depended on concrete `Arc<Vortex>` and `Arc<Gallifrey>`, creating high coupling and making testing difficult.
**Blueprint:**
1. Extracted domain types (`Entity`, `Message`, `Snapshot`, etc.) from `gallifrey` to `common/src/domain.rs`.
2. Updated `GallifreyService` trait in `common` to include methods for conversation and system state.
3. Refactored `Chronos` to use `Arc<dyn VortexService>` and `Arc<dyn GallifreyService>`.
**Stability:** `Chronos` is now decoupled from specific implementations, allowing for easier mocking and substitution.

**[Architect] Enforce Chronos Decoupling via Traits**
**Tangle:** `Chronos` was still coupled to concrete `Vortex` and `Gallifrey` structs despite previous plans. `Vortex` defined its own `ModelHandle`, causing type mismatches.
**Blueprint:**
1. Moved `ModelLoadConfig` and `InferenceParams` to `common::llm`.
2. Defined `VortexService` and `GallifreyService` in `common::traits`.
3. Refactored `vortex` to use `common::id::ModelHandle`.
4. Updated `Chronos` to use `Arc<dyn Service>`.
**Stability:** `Chronos` is now truly decoupled. `ModelHandle` is unified.

**[Refactor] Telemetry Types De-Bloat**
**Tangle:** `telemetry/src/types.rs` was a "Blob" (649 lines) mixing Tracing, Logging, Metrics, and Routing concerns.
**Blueprint:**
1. Extracted `trace.rs` (IDs), `log.rs` (Level), `meta.rs` (Subsystem), `wire.rs` (TelemetryEntry), and `metrics_types.rs`.
2. Converted `types.rs` into a Facade re-exporting these modules.
**Stability:** Improved cohesion. `no_std` compatibility preserved.

**[Refactor] Vortex Weights Loader De-Bloat**
**Tangle:** `vortex/src/loader/weights.rs` was a "Blob" (788 lines) handling SafeTensors, GGUF, and type definitions.
**Blueprint:**
1. Extracted `types.rs` for `ModelFormat`, `LoadedModel`.
2. Extracted `safetensors_loader.rs` and `gguf_loader.rs` for format-specific logic.
3. Retained `weights.rs` as a lightweight facade/dispatcher.
**Stability:** Reduced `weights.rs` to <100 lines. Improved separation of concerns.

**[Refactor] Consolidate QueryResult**
**Tangle:** `tardis_common::traits` contained a single struct `QueryResult` (which is not a trait) while `gallifrey` defined a conflicting, loosely-typed version.
**Blueprint:** Moved `QueryResult` to `gallifrey::query`, replacing the loose definition with the strictly typed one. Deleted the misnamed `common::traits` module.
**Stability:** Enforced domain boundaries and removed a misleading module.
**[Refactor] Unify Temporal References**
**Tangle:** "The Sprawl" - `chronos` defined its own `TemporalRef` while `common` had a nearly identical `TemporalReference`. This caused type mismatches and duplication.
**Blueprint:**
1. Enhanced `common::temporal::TemporalReference` with `PartialEq` and `Eq`.
2. Refactored `chronos::pipeline::analyzer` to use `common::temporal::TemporalReference`.
3. Updated consumers (`retriever`, `augmenter`) to use the shared type.
**Stability:** Removed duplicate types, enforced consistency across crates.

**[Architect] Decouple Telemetry from Std**
**Tangle:** `telemetry` crate depended unconditionally on `tardis-common` (std-only) and `chrono` (std-default), preventing `kernel` usage.
**Blueprint:**
1. Made `tardis-common` and `chrono` optional dependencies in `telemetry`.
2. Configured `serde` and `serde_json` for `no_std` environments.
3. Updated `std` feature to enable these dependencies.
**Stability:** Enables `telemetry` to be used in `kernel` (no_std) while retaining full functionality in userspace.

**[Architect] The Great Decoupling**
**Tangle:** `tardis-common` was becoming a "God Crate" (holding `Entity`, `Message`, `Snapshot`, `LLM`), causing domain leakage and weak cohesion. `Chronos` depended on `common` for things that belonged in `Gallifrey` or `Vortex`.
**Blueprint:**
1. Moved `common::domain` to `gallifrey::domain` (the Knowledge Store).
2. Moved `common::llm` to `vortex::config` (the Inference Engine).
3. Updated `common` to only contain shared primitives (`id`, `error`, `temporal`).
**Stability:** Enforced strict domain boundaries. `common` is now truly common.

**[Refactor] Centralized Error De-Bloat**
**Tangle:** `tardis_common::Error` was a "God Enum" containing all possible errors from `vortex`, `gallifrey`, `chronos`, and `shell`. This violated the Open/Closed Principle and created a dependency bottleneck.
**Blueprint:**
1. Removed domain-specific variants from `tardis_common::Error`.
2. Updated `gallifrey` to return `GallifreyResult<T>` instead of `tardis_common::Result<T>`.
3. Removed `impl From<DomainError> for tardis_common::Error` in `vortex` and `gallifrey`.
4. Updated `chronos` to handle crate-specific errors directly.
**Stability:** Enforced error boundaries. Adding a new error in a specific crate no longer requires modifying `common`.
