**[Enum-Driven Logic Testing]**
**Learning:** Table-driven tests are highly effective for verifying logic that branches on `enum` variants (like `QueryIntent`), ensuring every case yields the expected substring without redundant code.
**Action:** Use `vec![(Variant, "expected")]` loops for `match` statements in future tests.
