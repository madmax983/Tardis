# Warden's Journal

**2025-05-18 - [DoS via Large Inputs]**
**Threat:** User inputs to `query` and `remember` were unbounded, allowing a DoS attack via memory exhaustion or excessive processing (e.g. `to_lowercase` on large strings).
**Defense:** Enforced `MAX_INPUT_LEN` (10KB) limit on inputs in `chronos` pipeline.
