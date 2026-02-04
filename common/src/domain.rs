//! Domain entities shared across Tardis OS.

use crate::id::{EntityId, SessionId, SnapshotId};
use crate::temporal::BiTemporalInterval;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Knowledge Graph
// ============================================================================

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier.
    pub id: EntityId,
    /// Entity type (e.g., "Concept", "Person", "Project").
    pub entity_type: String,
    /// Entity name.
    pub name: String,
    /// Properties as key-value pairs.
    pub properties: HashMap<String, serde_json::Value>,
    /// Embedding vector for semantic search.
    pub embedding: Option<Vec<f32>>,
    /// Temporal metadata.
    pub temporal: BiTemporalInterval,
    /// Source of this knowledge.
    pub source: Option<String>,
}

/// A relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier.
    pub id: EntityId,
    /// Relationship type (e.g., "KNOWS", "CONTAINS", "`DEPENDS_ON`").
    pub relationship_type: String,
    /// Source entity ID.
    pub source: EntityId,
    /// Target entity ID.
    pub target: EntityId,
    /// Relationship properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Temporal metadata.
    pub temporal: BiTemporalInterval,
}

/// Query results from Gallifrey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result nodes.
    pub nodes: Vec<Entity>,
    /// Query execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Whether results were truncated.
    pub truncated: bool,
}

// ============================================================================
// Conversation
// ============================================================================

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// User message.
    User,
    /// Assistant response.
    Assistant,
    /// System message.
    System,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier.
    pub id: EntityId,
    /// Session this message belongs to.
    pub session_id: SessionId,
    /// Message role.
    pub role: Role,
    /// Message content.
    pub content: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Embedding for semantic search.
    pub embedding: Option<Vec<f32>>,
    /// References to knowledge graph entities.
    pub entity_refs: Vec<EntityId>,
}

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier.
    pub id: SessionId,
    /// Session start time.
    pub started_at: DateTime<Utc>,
    /// Session end time (None if active).
    pub ended_at: Option<DateTime<Utc>>,
    /// Session summary (generated after session ends).
    pub summary: Option<String>,
    /// Topics discussed.
    pub topics: Vec<String>,
    /// Session metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

// ============================================================================
// System State
// ============================================================================

/// A snapshot of system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique identifier.
    pub id: SnapshotId,
    /// Snapshot name (user-provided or auto-generated).
    pub name: String,
    /// When the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// What triggered the snapshot.
    pub trigger: SnapshotTrigger,
    /// Captured state.
    pub state: SystemState,
    /// Checksum for integrity.
    pub checksum: String,
}

/// What triggered a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotTrigger {
    /// Scheduled snapshot.
    Scheduled,
    /// User-requested snapshot.
    Manual,
    /// Before a significant operation.
    PreOperation(String),
    /// After an error.
    Error(String),
}

/// Captured system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// Process states.
    pub processes: HashMap<u32, ProcessState>,
    /// Configuration values.
    pub config: HashMap<String, serde_json::Value>,
    /// File system snapshot (paths and metadata).
    pub files: HashMap<String, FileMetadata>,
}

/// Process state information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessState {
    /// Process ID.
    pub pid: u32,
    /// Process name.
    pub name: String,
    /// Process status.
    pub status: String,
    /// Memory usage.
    pub memory_bytes: u64,
    /// CPU usage percentage.
    pub cpu_percent: f32,
}

/// File metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path.
    pub path: String,
    /// File size.
    pub size: u64,
    /// Last modified time.
    pub modified: DateTime<Utc>,
    /// File hash (for content tracking).
    pub hash: Option<String>,
}

/// A change between snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
    /// What changed.
    pub path: String,
    /// Type of change.
    #[allow(clippy::struct_field_names)]
    pub change_type: ChangeType,
    /// Old value (if applicable).
    pub old_value: Option<serde_json::Value>,
    /// New value (if applicable).
    pub new_value: Option<serde_json::Value>,
}

/// Type of change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Created.
    Create,
    /// Updated.
    Update,
    /// Deleted.
    Delete,
}
