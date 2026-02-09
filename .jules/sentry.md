**[RingBuffer Atomic Wrapping Bug]
**Learning:** `AtomicUsize` wrapping behavior combined with `saturating_sub` caused incorrect `available()` counts when `write_pos` wrapped but `read_pos` hadn't. Lock-free ring buffers must use `wrapping_sub` for position arithmetic to respect the circular nature of the sequence.
**Action:** Always test wrapping arithmetic with explicit `usize::MAX` boundary conditions using `unsafe` pointer manipulation if necessary to simulate long-running states.

**[Large Struct Testing]
**Learning:** `Box::new(LargeStruct::new())` often overflows the stack because Rust constructs the value on the stack before moving it. For large structs like `RingBuffer` (1MB), using `static` with a `reset()` helper is a reliable pattern for property-based testing.
**Action:** Use `static` buffers + reset logic instead of heap allocation for testing large `const fn` initialized structs.

**[RAG Context Truncation]
**Learning:** Naive token limit checks that discard entire sources when they overflow can severely limit context utilization, especially with large documents or small token budgets. Partial truncation (e.g., ) allows filling the remaining budget effectively.
**Action:** When implementing token-limited buffers, always implement partial filling logic rather than binary include/exclude decisions.

**[RAG Context Truncation]
**Learning:** Naive token limit checks that discard entire sources when they overflow can severely limit context utilization, especially with large documents or small token budgets. Partial truncation (e.g., `chars().take()`) allows filling the remaining budget effectively.
**Action:** When implementing token-limited buffers, always implement partial filling logic rather than binary include/exclude decisions.
