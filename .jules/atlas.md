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
