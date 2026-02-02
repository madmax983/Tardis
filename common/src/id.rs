//! Identifier types for Tardis OS.
//!
//! # Why Strong Typing?
//!
//! This module provides strongly-typed identifiers for various entities to prevent
//! accidental mixing of different ID types at compile time. For example, it prevents
//! passing a [`SessionId`] to a function that expects an [`EntityId`], even though
//! both might inherently be UUIDs.
//!
//! # Overview
//!
//! - [`ModelHandle`]: A lightweight, `Copy`able handle to a loaded model in Vortex.
//! - [`EntityId`]: Identity for a node in the Gallifrey knowledge graph.
//! - [`SessionId`]: Tracks a conversational context in Chronos.
//! - [`MessageId`]: Identifies a single message within a session.
//! - [`SnapshotId`]: Points to an immutable state of the knowledge graph.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// A handle to a loaded LLM model in Vortex.
///
/// Handles are lightweight 64-bit integers that map to loaded model instances.
/// They are designed to be passed around cheaply.
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
    ///
    /// # Arguments
    ///
    /// * `value` - The raw 64-bit integer identifier.
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
/// Entities are the nodes in Gallifrey's graph. Each entity has a globally
/// unique identifier that persists across versions and time.
///
/// # Examples
///
/// ```
/// use tardis_common::id::EntityId;
///
/// let id = EntityId::new();
/// println!("Created entity: {}", id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(Uuid);

impl EntityId {
    /// Create a new random entity ID.
    ///
    /// Uses a V4 UUID to ensure global uniqueness.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an entity ID from an existing UUID.
    ///
    /// Use this when deserializing or recovering an ID from storage.
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
/// A session groups a series of messages and context.
///
/// # Examples
///
/// ```
/// use tardis_common::id::SessionId;
///
/// let session = SessionId::new();
/// // Pass this session ID to Chronos to append messages
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a session ID from an existing UUID.
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
/// Snapshots represent a frozen point in time of the knowledge graph,
/// useful for consistent queries or backups.
///
/// # Examples
///
/// ```
/// use tardis_common::id::SnapshotId;
///
/// let snapshot = SnapshotId::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(Uuid);

impl SnapshotId {
    /// Create a new random snapshot ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a snapshot ID from an existing UUID.
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
///
/// Every message sent by a user or generated by the system is assigned
/// a unique ID for referencing and history tracking.
///
/// # Examples
///
/// ```
/// use tardis_common::id::MessageId;
///
/// let msg_id = MessageId::new();
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
    fn session_id_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let id = SessionId::new();
        let json = serde_json::to_string(&id)?;
        let parsed: SessionId = serde_json::from_str(&json)?;
        assert_eq!(id, parsed);
        Ok(())
    }
}
