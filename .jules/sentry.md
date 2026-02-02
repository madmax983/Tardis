**[Debug Impl Strategy]**
**Learning:** When `#[derive(Debug)]` fails because a field (e.g., `Arc<T>`) points to a type `T` (like `Gallifrey`) that lacks `Debug`, manually implementing `fmt::Debug` for the struct allows satisfying `missing_debug_implementations` without refactoring the dependency chain.
**Action:** Use manual `impl fmt::Debug` and exclude or placeholder-print non-Debug fields to maintain code hygiene in the current crate.
