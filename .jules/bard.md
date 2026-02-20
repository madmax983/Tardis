## 2024-05-22 - The Invisible Front Door
**Confusion:** The project had no root `README.md`. It was impossible to know what the project was or how to build it without digging into `docs/` or `Cargo.toml`.
**Clarification:** Created a root `README.md` that acts as a portal to the detailed documentation and provides a high-level overview of the crates.

## 2024-05-24 - The Time Travel Paradox
**Confusion:** The difference between `valid_time` and `transaction_time` in `BiTemporalInterval` was opaque, leading to confusion about how to "correct" history versus "record" it.
**Clarification:** Added a dedicated "Bi-temporality Explained" section with a concrete example of `supersede()` showing how to correct a past mistake without losing the audit trail.

## 2024-05-25 - The Hidden Chronos
**Confusion:** Users exploring the `chronos` crate had no entry point (`README.md`) to understand the RAG pipeline architecture, making the orchestration logic feel like a black box.
**Clarification:** Added `chronos/README.md` with an ASCII architecture diagram and detailed pipeline stage explanations to map the flow from query to response.

## 2024-05-26 - The Randomness Requirement
**Confusion:** `TraceId::generate()` is often assumed to be available everywhere, but it requires `std::time`, making it unavailable in `no_std` kernel builds.
**Clarification:** Explicitly guarded `generate()` examples with `#[cfg(feature = "std")]` and explained that `from_bytes` is the `no_std` alternative.

## 2024-05-27 - The Silent Truncation
**Confusion:** RAG responses were occasionally missing context without error, due to the `ContextAugmenter` silently truncating sources that exceeded the token budget.
**Clarification:** Added a "Token Budgeting" section to `chronos/src/pipeline/augmenter.rs` explaining the 4-char heuristic and truncation logic.

## 2026-02-13 - The Mock Mirage
**Confusion:** Developers expecting full RAG pipeline functionality were confused by empty or static responses from `chronos::query`.
**Clarification:** Explicitly documented `chronos::query` as using a mock inference engine until `vortex` integration is complete.

## 2026-02-13 - The Bi-Temporal Blind Spot
**Confusion:** Users struggled to understand why `update()` created a new version instead of overwriting data, leading to "ghost" data from the past.
**Clarification:** Expanded the "Bi-Temporality" section in `gallifrey` docs to explain the "Append-Only" nature of updates and how to query valid vs. transaction time.

## 2026-06-15 - The Linear Scan Trap
**Confusion:** Developers assumed `Gallifrey` had an index for entity names, leading to O(N) performance issues when looking up entities by name.
**Clarification:** Documented `KnowledgeStore`'s lack of a name index and the cost of `scan_history`, warning users to rely on `EntityId` for fast lookups.
