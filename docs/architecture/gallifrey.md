# Gallifrey - Temporal Knowledge Store

## Overview

Gallifrey is Tardis's temporal storage layer, built on GallifreyDB. It provides bi-temporal data storage with three specialized stores for knowledge, conversations, and system state.

## Bi-Temporal Model

Gallifrey tracks two independent time dimensions:

```
                    Transaction Time →
                    (when recorded)

Valid Time ↓        ┌─────┬─────┬─────┬─────┐
(when true)         │ v1  │ v1  │ v2  │ v2  │  ← Current view
                    ├─────┼─────┼─────┼─────┤
                    │ v1  │ v1  │ v1  │ v2  │
                    ├─────┼─────┼─────┼─────┤
                    │     │ v1  │ v1  │ v1  │
                    └─────┴─────┴─────┴─────┘
                    t1    t2    t3    t4
```

- **Valid Time**: When the fact was true in the real world
- **Transaction Time**: When the fact was recorded in the database

This enables queries like:
- "What did I know about X at time T?" (point-in-time query)
- "What was my understanding of X last week?" (as-of query)
- "How has my knowledge of X changed?" (temporal diff)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       GALLIFREY                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                Knowledge Graph Store                   │  │
│  │                                                        │  │
│  │  Entities ──────── Relationships ──────── Facts        │  │
│  │      │                   │                   │         │  │
│  │      ▼                   ▼                   ▼         │  │
│  │  Embeddings         Properties           Sources       │  │
│  │                                                        │  │
│  │  [Bi-temporal versioning for all nodes/edges]          │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                Conversation Store                      │  │
│  │                                                        │  │
│  │  Sessions ──────── Messages ──────── Summaries         │  │
│  │      │                 │                  │            │  │
│  │      ▼                 ▼                  ▼            │  │
│  │  Metadata          Embeddings         Context          │  │
│  │                                                        │  │
│  │  [Cross-session continuity, temporal retrieval]        │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │               System State Journal                     │  │
│  │                                                        │  │
│  │  Snapshots ──────── Deltas ──────── Audit Log          │  │
│  │      │                 │                  │            │  │
│  │      ▼                 ▼                  ▼            │  │
│  │  Full State       Incremental       Who/What/When      │  │
│  │                                                        │  │
│  │  [Time-travel debugging, forensic analysis]            │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Knowledge Graph Store

### Schema

```rust
pub struct Entity {
    pub id: EntityId,
    pub entity_type: EntityType,
    pub name: String,
    pub properties: HashMap<String, Value>,
    pub embedding: Vec<f32>,
    pub valid_time: BiTemporalInterval,
}

pub struct Relationship {
    pub id: RelationshipId,
    pub relationship_type: RelationType,
    pub source: EntityId,
    pub target: EntityId,
    pub properties: HashMap<String, Value>,
    pub valid_time: BiTemporalInterval,
}

pub struct BiTemporalInterval {
    /// When this fact was/is true in reality
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    /// When this fact was recorded (system-managed)
    pub transaction_from: DateTime<Utc>,
    pub transaction_to: Option<DateTime<Utc>>,
}
```

### Example Data

```
(User:mark) ──[KNOWS]──▶ (Concept:Rust)
                              │
                              ├──[HAS_FEATURE]──▶ (Concept:Ownership)
                              │
                              └──[HAS_FEATURE]──▶ (Concept:Borrowing)

Each node/edge has:
- Properties: {understanding_level: "intermediate", ...}
- Embedding: [0.1, -0.2, 0.5, ...]
- Valid time: 2024-01-15 to present
- Transaction time: 2024-01-15T10:30:00Z to present
```

### Queries

```rust
// Find all concepts the user knows about
gallifrey.query("
    MATCH (u:User {name: 'mark'})-[:KNOWS]->(c:Concept)
    RETURN c
")?;

// Find concepts known at a specific time
gallifrey.query_at_time("
    MATCH (u:User)-[:KNOWS]->(c:Concept)
    RETURN c
", valid_time: "2024-03-01")?;

// Find how knowledge changed
gallifrey.query_changes(
    entity: "Rust",
    from: "2024-01-01",
    to: "2024-03-01",
)?;
```

## Conversation Store

### Schema

```rust
pub struct Session {
    pub id: SessionId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub topics: Vec<String>,
}

pub struct Message {
    pub id: MessageId,
    pub session: SessionId,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub embedding: Vec<f32>,
    pub references: Vec<EntityId>, // Links to knowledge graph
}

pub enum Role {
    User,
    Assistant,
    System,
}

pub struct ConversationContext {
    pub recent_messages: Vec<Message>,
    pub relevant_history: Vec<Message>,
    pub active_topics: Vec<String>,
}
```

### Cross-Session Continuity

```rust
impl ConversationStore {
    /// Find messages relevant to current query across all sessions
    pub fn find_relevant(&self, query: &str, limit: usize) -> Vec<Message> {
        let query_embedding = self.embed(query);

        self.messages
            .iter()
            .map(|m| (m, cosine_similarity(&query_embedding, &m.embedding)))
            .filter(|(_, score)| *score > 0.7)
            .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
            .take(limit)
            .map(|(m, _)| m.clone())
            .collect()
    }

    /// Get conversation context for RAG
    pub fn get_context(&self, query: &str, session: SessionId) -> ConversationContext {
        ConversationContext {
            recent_messages: self.recent_messages(session, 10),
            relevant_history: self.find_relevant(query, 5),
            active_topics: self.active_topics(session),
        }
    }
}
```

## System State Journal

### Schema

```rust
pub struct StateSnapshot {
    pub id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub trigger: SnapshotTrigger,
    pub state: SystemState,
    pub checksum: [u8; 32],
}

pub enum SnapshotTrigger {
    Scheduled,
    BeforeOperation(OperationId),
    Manual,
    Error(ErrorId),
}

pub struct StateDelta {
    pub id: DeltaId,
    pub from_snapshot: SnapshotId,
    pub to_snapshot: SnapshotId,
    pub changes: Vec<Change>,
}

pub struct Change {
    pub path: String,
    pub change_type: ChangeType,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub actor: ActorId,
    pub timestamp: DateTime<Utc>,
}

pub enum ChangeType {
    Create,
    Update,
    Delete,
}

pub struct AuditEntry {
    pub id: AuditId,
    pub timestamp: DateTime<Utc>,
    pub actor: ActorId,
    pub action: String,
    pub target: String,
    pub details: HashMap<String, Value>,
    pub result: AuditResult,
}
```

### Time-Travel Debugging

```rust
impl SystemStateJournal {
    /// Reconstruct system state at a point in time
    pub fn time_travel(&self, timestamp: DateTime<Utc>) -> SystemState {
        // Find nearest snapshot before timestamp
        let snapshot = self.find_snapshot_before(timestamp)?;

        // Apply deltas up to timestamp
        let mut state = snapshot.state.clone();
        for delta in self.deltas_after(snapshot.id, timestamp) {
            state.apply_delta(&delta);
        }

        state
    }

    /// Find what changed between two points in time
    pub fn diff(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Change> {
        let from_state = self.time_travel(from);
        let to_state = self.time_travel(to);

        from_state.diff(&to_state)
    }

    /// Find who/what caused a specific state
    pub fn trace_change(&self, path: &str, value: &Value) -> Option<AuditEntry> {
        self.audit_log
            .iter()
            .rev()
            .find(|e| e.target == path && e.details.get("new_value") == Some(value))
    }
}
```

## Query Engine

### Temporal Query Syntax

```rust
pub struct TemporalQuery {
    /// Base graph query
    pub query: String,
    /// Point-in-time for valid time
    pub valid_at: Option<DateTime<Utc>>,
    /// Point-in-time for transaction time
    pub transaction_at: Option<DateTime<Utc>>,
    /// Time range for valid time
    pub valid_between: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Include historical versions
    pub include_history: bool,
}

// Example queries:

// Current state (default)
"MATCH (n:Concept) RETURN n"

// State as of valid time
"MATCH (n:Concept) RETURN n AS OF VALID TIME '2024-03-01'"

// State as of transaction time (what we knew then)
"MATCH (n:Concept) RETURN n AS OF SYSTEM TIME '2024-03-01T10:00:00Z'"

// Both dimensions
"MATCH (n:Concept) RETURN n
 AS OF VALID TIME '2024-03-01'
 AS OF SYSTEM TIME '2024-03-15T10:00:00Z'"

// Historical versions
"MATCH (n:Concept) RETURN n WITH HISTORY"
```

### Index Structures

```rust
pub struct TemporalIndex {
    /// B-tree on valid_from for range queries
    valid_time_index: BTreeIndex<DateTime<Utc>, EntityId>,
    /// B-tree on transaction_from for as-of queries
    transaction_time_index: BTreeIndex<DateTime<Utc>, EntityId>,
    /// HNSW index on embeddings for vector search
    embedding_index: HnswIndex<EntityId>,
    /// Full-text index for content search
    text_index: TantivyIndex,
}
```

## Syscall Interface

```rust
/// Execute temporal query
pub fn sys_gallifrey_query(
    query: *const u8,
    query_len: usize,
    temporal_params: *const TemporalParams,
    result: *mut QueryResult,
) -> isize;

/// Insert entity/relationship
pub fn sys_gallifrey_insert(
    data: *const GraphData,
    valid_time: *const ValidTime,
) -> isize;

/// Update entity/relationship
pub fn sys_gallifrey_update(
    id: EntityId,
    changes: *const Changes,
    valid_time: *const ValidTime,
) -> isize;

/// Time-travel to reconstruct past state
pub fn sys_gallifrey_time_travel(
    timestamp: *const Timestamp,
    snapshot: *mut Snapshot,
) -> isize;

/// Get history of entity changes
pub fn sys_gallifrey_get_history(
    entity_id: EntityId,
    timeline: *mut Timeline,
    capacity: usize,
) -> isize;
```

## Directory Structure

```
gallifrey/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── stores/
    │   ├── mod.rs
    │   ├── knowledge.rs      # Knowledge graph store
    │   ├── conversation.rs   # Conversation store
    │   └── system_state.rs   # System state journal
    ├── temporal/
    │   ├── mod.rs
    │   ├── bi_temporal.rs    # Bi-temporal primitives
    │   ├── time_travel.rs    # Time-travel operations
    │   └── versioning.rs     # Version management
    ├── query/
    │   ├── mod.rs
    │   ├── parser.rs         # Query parser
    │   ├── planner.rs        # Query planner
    │   └── executor.rs       # Query executor
    └── index/
        ├── mod.rs
        ├── temporal.rs       # Temporal indexes
        ├── embedding.rs      # Vector indexes
        └── text.rs           # Full-text indexes
```

## Storage Format

GallifreyDB uses an append-only log with anchor+delta compression:

```
┌─────────────────────────────────────────────────────────────┐
│                    Append-Only Log                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌───┐  ┌───┐  ┌─────────┐  ┌───┐  ┌───┐      │
│  │ Anchor  │  │ Δ │  │ Δ │  │ Anchor  │  │ Δ │  │ Δ │ ...  │
│  │ (full)  │  │ 1 │  │ 2 │  │ (full)  │  │ 3 │  │ 4 │      │
│  └─────────┘  └───┘  └───┘  └─────────┘  └───┘  └───┘      │
└─────────────────────────────────────────────────────────────┘
```

- **Anchors**: Full state snapshots, created periodically
- **Deltas**: Incremental changes between anchors
- **5-6x compression** compared to storing full versions
