//! Identifier types for Tardis OS.
//!
//! Provides strongly-typed identifiers for various entities to prevent
//! mixing up different ID types at compile time.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A handle to a loaded LLM model in Vortex.
///
/// # Examples
///
/// ```
/// use tardis_common::id::ModelHandle;
///
/// let handle = ModelHandle::new(42);
/// assert_eq!(handle.raw(), 42);
/// assert_eq!(handle.to_string(), "model:42");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelHandle(u64);

impl ModelHandle {
    /// Create a new model handle from a raw value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw handle value.
    #[must_use]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ModelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "model:{}", self.0)
    }
}

/// Unique identifier for an entity in the knowledge graph.
///
/// # Examples
///
/// Creating a new random ID:
/// ```
/// use tardis_common::id::EntityId;
///
/// let id = EntityId::new();
/// println!("New entity: {}", id); // "entity:..."
/// ```
///
/// Creating from a known UUID:
/// ```
/// use tardis_common::id::EntityId;
/// use uuid::Uuid;
///
/// let uuid = Uuid::nil();
/// let id = EntityId::from_uuid(uuid);
/// assert_eq!(id.as_uuid(), uuid);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(Uuid);

impl EntityId {
    /// Create a new random entity ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an entity ID from a UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "entity:{}", self.0)
    }
}

/// Unique identifier for a conversation session.
///
/// # Examples
///
/// ```
/// use tardis_common::id::SessionId;
///
/// let session_id = SessionId::new();
/// println!("Current session: {}", session_id); // "session:..."
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a session ID from a UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session:{}", self.0)
    }
}

/// Unique identifier for a system state snapshot.
///
/// # Examples
///
/// ```
/// use tardis_common::id::SnapshotId;
///
/// let snap_id = SnapshotId::new();
/// let max_id = SnapshotId::max();
///
/// assert!(snap_id < max_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SnapshotId(Uuid);

impl SnapshotId {
    /// Create a new random snapshot ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a snapshot ID from a UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Create a maximum possible snapshot ID.
    ///
    /// Useful for range queries.
    #[must_use]
    pub const fn max() -> Self {
        Self(Uuid::from_u128(u128::MAX))
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "snapshot:{}", self.0)
    }
}

/// Unique identifier for a message in conversation history.
///
/// # Examples
///
/// ```
/// use tardis_common::id::MessageId;
///
/// let msg_id = MessageId::new();
/// println!("Message ref: {}", msg_id); // "msg:..."
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    /// Create a new random message ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "msg:{}", self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn model_handle_display() {
        let handle = ModelHandle::new(42);
        assert_eq!(handle.to_string(), "model:42");
        assert_eq!(handle.raw(), 42);
    }

    #[test]
    fn entity_id_unique() {
        let id1 = EntityId::new();
        let id2 = EntityId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn session_id_serialization() {
        let id = SessionId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
