**Iterator Overhead on Hot Paths
**Learning:** `haystack.as_bytes().windows(n).any(...)` is elegant but introduces significant overhead for short strings compared to manual loops, especially when checking against multiple needles.
**Action:** Use manual byte loops with a "first-byte fast check" for high-frequency string matching in `no_std` or performance-critical contexts.

**Cow in Public Structs
**Learning:** Replacing `String` with `Cow<'static, str>` in a public struct is a breaking API change because it breaks existing struct literal initialization (e.g., `Struct { field: string_val }` fails type inference/conversion).
**Action:** Avoid changing field types in public structs unless a major version bump is planned. Use internal storage optimizations or new types instead.

**String Allocation during Truncation
**Learning:** `str::chars().take(n).collect::<String>()` allocates a new `String` which is immediately discarded if used only for display/formatting.
**Action:** Use `str::char_indices().nth(n)` to find the split point and use a string slice `&str[..idx]` to avoid allocation.
