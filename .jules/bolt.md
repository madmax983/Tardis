**Iterator Overhead on Hot Paths
**Learning:** `haystack.as_bytes().windows(n).any(...)` is elegant but introduces significant overhead for short strings compared to manual loops, especially when checking against multiple needles.
**Action:** Use manual byte loops with a "first-byte fast check" for high-frequency string matching in `no_std` or performance-critical contexts.
**Hoisting Invariants
**Learning:** `to_lowercase()` allocates a new String. Calling it multiple times in a pipeline for the same input is wasteful. Hoisting it to the top level saved ~50% execution time in query analysis.
**Action:** Always compute derived invariants (like lowercase, trimmed) once at the entry point and pass references down.
