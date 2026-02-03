# Sentry's Journal

**[Dynamic Time in Tests]**
**Learning:** Testing functions that use `Utc::now()` is tricky. Instead of mocking the entire time system for simple checks, verifying the presence of the label (e.g., "Current time:") is often sufficient for ensuring the field is being rendered.
**Action:** Use broad assertions for dynamic fields unless exact values are critical logic.

**[Benchmark Black Box]**
**Learning:** `criterion::black_box` is deprecated and causes warnings. `std::hint::black_box` is the standard replacement.
**Action:** Always use `std::hint::black_box` in benchmarks.
