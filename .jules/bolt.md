**[Cow Optimization for Case-Insensitive Parsing]
**Learning:** `str::to_lowercase()` creates a heap allocation every time. In hot paths like telemetry target parsing where inputs are predominantly lowercase (e.g., module paths), this overhead is significant.
**Action:** Use `Cow<'a, str>` combined with a check like `target.chars().any(char::is_uppercase)` to borrow the original string in the common case, allocating only when necessary.
