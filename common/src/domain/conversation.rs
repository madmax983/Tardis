//! Conversation entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::id::{EntityId, SessionId};

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
///
/// Represents a single turn in a dialogue between the user and the system.
///
/// # Examples
///
/// ```
/// use tardis_common::domain::{Message, Role};
/// use tardis_common::id::{EntityId, SessionId};
/// use chrono::Utc;
///
/// let message = Message {
///     id: EntityId::new(),
///     session_id: SessionId::new(),
///     role: Role::User,
///     content: "Explain the bootstrap paradox.".to_string(),
///     timestamp: Utc::now(),
///     embedding: None,
///     entity_refs: vec![],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message.
    pub id: EntityId,
    /// The session this message belongs to.
    ///
    /// Messages are grouped by session to maintain conversation context.
    pub session_id: SessionId,
    /// The role of the speaker (User, Assistant, System).
    pub role: Role,
    /// The text content of the message.
    pub content: String,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
    /// Embedding vector for semantic search over chat history.
    ///
    /// Allows finding past conversations about specific topics even if keywords differ.
    pub embedding: Option<Vec<f32>>,
    /// IDs of entities mentioned or relevant to this message.
    ///
    /// Linked by the `Chronos` analysis pipeline to connect chat to the knowledge graph.
    pub entity_refs: Vec<EntityId>,
}

/// A conversation session.
///
/// Tracks the lifecycle of a user interaction, including topics discussed
/// and metadata for context retrieval.
///
/// # Examples
///
/// ```
/// use tardis_common::domain::Session;
/// use tardis_common::id::SessionId;
/// use chrono::Utc;
/// use std::collections::HashMap;
///
/// let session = Session {
///     id: SessionId::new(),
///     started_at: Utc::now(),
///     ended_at: None,
///     summary: None,
///     topics: vec!["Physics".to_string(), "Time Travel".to_string()],
///     metadata: HashMap::new(),
/// };
/// ```
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
