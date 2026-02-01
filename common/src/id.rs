//! Identifier types for Tardis OS.
//!
//! Provides strongly-typed identifiers for various entities to prevent
//! mixing up different ID types at compile time.
//!
//! # Why?
//!
//! In a complex system like Tardis, passing around raw `Uuid`s or `u64`s is a recipe for bugs.
//! For example, you might accidentally pass a `SessionId` to a function expecting an `EntityId`.
//! By using the "Newtype" pattern, the compiler catches these errors for us.
//!
//! # Examples
//!
//! ```rust
//! use tardis_common::id::{EntityId, SessionId};
//!
//! // Creating IDs
//! let entity_id = EntityId::new();
//! let session_id = SessionId::new();
//!
//! // This would be a compile error:
//! // fn process_entity(id: EntityId) {}
//! // process_entity(session_id);
//!
//! // IDs have helpful string representations
//! assert!(entity_id.to_string().starts_with("entity:"));
//! assert!(session_id.to_string().starts_with("session:"));
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A handle to a loaded LLM model in Vortex.
///
/// Handles are lightweight `u64` values that map to loaded model weights in GPU memory.
///
/// # Examples
///
/// ```
/// use tardis_common::id::ModelHandle;
///
/// let handle = ModelHandle::new(101);
/// assert_eq!(handle.raw(), 101);
/// assert_eq!(handle.to_string(), "model:101");
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
/// Entities are the fundamental nodes in the Gallifrey knowledge graph.
/// Each entity is identified by a UUID.
///
/// # Examples
///
/// ```
/// use tardis_common::id::EntityId;
///
/// let id = EntityId::new();
/// println!("Created entity: {}", id); // Prints "entity:<uuid>"
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
/// A session represents a continuous interaction thread with the user.
///
/// # Examples
///
/// ```
/// use tardis_common::id::SessionId;
///
/// let id = SessionId::new();
/// println!("Current session: {}", id); // Prints "session:<uuid>"
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
/// Snapshots capture the state of the OS at a specific point in time
/// for time-travel debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
