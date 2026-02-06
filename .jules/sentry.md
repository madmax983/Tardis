**[RingBuffer Atomic Wrapping Bug]
**Learning:** `AtomicUsize` wrapping behavior combined with `saturating_sub` caused incorrect `available()` counts when `write_pos` wrapped but `read_pos` hadn't. Lock-free ring buffers must use `wrapping_sub` for position arithmetic to respect the circular nature of the sequence.
**Action:** Always test wrapping arithmetic with explicit `usize::MAX` boundary conditions using `unsafe` pointer manipulation if necessary to simulate long-running states.

**[Large Struct Testing]
**Learning:** `Box::new(LargeStruct::new())` often overflows the stack because Rust constructs the value on the stack before moving it. For large structs like `RingBuffer` (1MB), using `static` with a `reset()` helper is a reliable pattern for property-based testing.
**Action:** Use `static` buffers + reset logic instead of heap allocation for testing large `const fn` initialized structs.
