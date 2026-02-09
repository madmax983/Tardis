**[Optimization] SystemStateStore Snapshot Lookup**
**Learning:** `HashMap::values().filter().max_by_key()` is O(N) and expensive for large datasets. Maintaining a secondary `BTreeMap` index allows O(log N) lookups for temporal queries.
**Action:** Always consider access patterns. If range or nearest-neighbor queries are needed, use a sorted collection (BTreeMap) as an index.
