# ADR-004: GallifreyDB for Temporal Storage

## Status

Accepted

## Date

2024-12-31

## Context

Tardis requires a storage system that can:
- Store knowledge graphs with semantic embeddings
- Track conversation history across sessions
- Record system state for time-travel debugging
- Support bi-temporal queries ("What did I know about X as of time T?")
- Efficiently compress historical data

We need to choose a database that supports these temporal requirements.

## Decision

We will use GallifreyDB as the temporal storage layer for all three stores (knowledge, conversation, system state).

## Consequences

### Positive

- **Bi-temporal native**: GallifreyDB is designed from the ground up for bi-temporal data (valid time + transaction time)
- **Rust native**: Written in Rust, integrates seamlessly with Tardis codebase
- **Graph model**: Property graph model suits knowledge representation and relationships
- **Anchor+Delta compression**: 5-6x storage reduction while maintaining query performance
- **Sub-microsecond queries**: Designed for fast single-hop queries, sub-100μs 3-hop traversals
- **Same author**: Built by the same developer, ensuring tight integration and support

### Negative

- **Early stage**: GallifreyDB is in "Core Foundation" phase, may have gaps
- **Limited ecosystem**: No existing tooling, ORMs, or community resources
- **Vector search TBD**: May need to add embedding/vector index support
- **Single project dependency**: Heavy reliance on one external project

### Neutral

- May need to contribute features back to GallifreyDB
- Documentation will grow with Tardis development
- Could become a reference implementation for GallifreyDB usage

## Alternatives Considered

### Alternative 1: PostgreSQL + TimescaleDB

Use PostgreSQL with TimescaleDB extension for temporal data.

**Pros:**
- Mature, battle-tested database
- TimescaleDB provides time-series capabilities
- Rich ecosystem of tools and ORMs
- pgvector for embedding storage

**Cons:**
- Not designed for bi-temporal data (requires complex schema design)
- Heavy dependency (full PostgreSQL installation)
- Graph queries require recursive CTEs or separate extension
- Not Rust-native

**Why not chosen:** PostgreSQL can simulate temporal features but isn't designed for them. The complexity of bi-temporal queries in SQL would be significant.

### Alternative 2: SurrealDB

Use SurrealDB for its multi-model capabilities.

**Pros:**
- Rust-native
- Graph, document, and relational in one
- Built-in record versioning
- Embeddable

**Cons:**
- No true bi-temporal support
- Versioning is not the same as temporal queries
- Younger project with changing APIs

**Why not chosen:** While SurrealDB has versioning, it lacks the bi-temporal query model (valid time vs transaction time) that Tardis requires.

### Alternative 3: Custom Storage Layer

Build a custom temporal storage layer from scratch.

**Pros:**
- Complete control over design
- Optimized specifically for Tardis use cases
- No external dependencies

**Cons:**
- Massive development effort
- Reinventing complex solved problems
- Time better spent on AI features

**Why not chosen:** Building a temporal database from scratch would distract from Tardis's core value proposition.

### Alternative 4: Datomic-like Append-Only Store

Implement a Datomic-inspired immutable append-only store.

**Pros:**
- Naturally temporal (all history preserved)
- Simple model (entity-attribute-value-time)
- Datalog queries

**Cons:**
- Would need to build from scratch
- Datalog learning curve
- No existing Rust implementation

**Why not chosen:** While architecturally appealing, building this from scratch is too much effort when GallifreyDB already provides temporal graph capabilities.

## References

- [GallifreyDB Repository](https://github.com/madmax983/GallifreyDB)
- [Bi-temporal Data Management](https://en.wikipedia.org/wiki/Temporal_database)
- [Anchor+Delta Compression](https://www.vldb.org/pvldb/vol13/p2991-wu.pdf)
