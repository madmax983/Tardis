//! System state entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tardis_common::id::SnapshotId;

/// A snapshot of system state.
///
/// Captures a frozen point-in-time view of the OS, allowing for
/// time-travel debugging and rollback.
///
/// # Examples
///
/// ```
/// use tardis_gallifrey::domain::{Snapshot, SnapshotTrigger, SystemState};
/// use tardis_common::id::SnapshotId;
/// use chrono::Utc;
/// use std::collections::HashMap;
///
/// let snapshot = Snapshot {
///     id: SnapshotId::new(),
///     name: "Pre-Update Backup".to_string(),
///     timestamp: Utc::now(),
///     trigger: SnapshotTrigger::Manual,
///     state: SystemState {
///         processes: HashMap::new(),
///         config: HashMap::new(),
///         files: HashMap::new(),
///     },
///     checksum: "sha256:abc123...".to_string(),
/// };
/// ```
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
///
/// Represents a diff operation on the system state, used for
/// audit logging and visualizing evolution over time.
///
/// # Examples
///
/// ```
/// use tardis_gallifrey::domain::{Change, ChangeType};
/// use chrono::Utc;
/// use serde_json::json;
///
/// let change = Change {
///     timestamp: Utc::now(),
///     path: "/etc/tardis/config.toml".to_string(),
///     change_type: ChangeType::Update,
///     old_value: Some(json!({ "debug": false })),
///     new_value: Some(json!({ "debug": true })),
/// };
/// ```
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
