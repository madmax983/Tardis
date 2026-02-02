# Atlas Journal

**[Refactor] Vortex Tokenizer De-Bloat**
**Tangle:** `vortex/src/tokenizer/mod.rs` was a "Blob" (747 lines) containing Service logic, Types, and Chat Template logic. It violated Single Responsibility Principle.
**Blueprint:** Extracted `types.rs` for data structures and `template.rs` for template logic.
**Stability:** Reduced `mod.rs` size, improved cohesion, and enforced strict visibility (`pub(crate) mod template`).
