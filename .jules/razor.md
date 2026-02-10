## [Reduction]
**Bloat:** `tardis_common` acting as a "God Object" dumping ground for domain entities (`Entity`, `Message`) and LLM configuration (`ModelLoadConfig`).
**Cut:** Moved domain entities to `tardis_gallifrey::domain` (where they are stored) and LLM config to `tardis_vortex::config` (where they are used). Deleted `common/src/domain.rs` and `common/src/llm.rs`.
**Saved:** Reduced `tardis_common` scope to truly shared primitives (`id`, `error`, `temporal`). Improved compilation times by reducing unnecessary rebuilds of `common` when domain/config changes.
