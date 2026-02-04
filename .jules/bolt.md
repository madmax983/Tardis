**Iterator Overhead on Hot Paths
**Learning:** `haystack.as_bytes().windows(n).any(...)` is elegant but introduces significant overhead for short strings compared to manual loops, especially when checking against multiple needles.
**Action:** Use manual byte loops with a "first-byte fast check" for high-frequency string matching in `no_std` or performance-critical contexts.
