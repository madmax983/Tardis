## 2024-05-22 - The Invisible Front Door
**Confusion:** The project had no root `README.md`. It was impossible to know what the project was or how to build it without digging into `docs/` or `Cargo.toml`.
**Clarification:** Created a root `README.md` that acts as a portal to the detailed documentation and provides a high-level overview of the crates.

## 2024-05-24 - The Time Travel Paradox
**Confusion:** The difference between `valid_time` and `transaction_time` in `BiTemporalInterval` was opaque, leading to confusion about how to "correct" history versus "record" it.
**Clarification:** Added a dedicated "Bi-temporality Explained" section with a concrete example of `supersede()` showing how to correct a past mistake without losing the audit trail.
