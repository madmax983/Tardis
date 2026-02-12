## [Reduction]
**Bloat:** Stateless structs (`QueryAnalyzer`, `CommandHandler`, `Router`) acting as namespaces for pure functions.
**Cut:** Converted to modules with free functions.
**Saved:** Removed 3 structs, 3 `new()` methods, 3 fields in consumer structs, and simplified call sites. Reduced cognitive load by making execution path explicit (functions are stateless).
