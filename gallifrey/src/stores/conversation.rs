//! Conversation history store.
//!
//! Stores chat sessions and messages with:
//! - Cross-session continuity
//! - Semantic search across history
//! - Session summaries

use crate::error::{GallifreyError, GallifreyResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tardis_common::{EntityId, SessionId};

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

/// The conversation store.
pub struct ConversationStore {
    /// Sessions indexed by ID.
    sessions: RwLock<HashMap<SessionId, Session>>,
    /// Messages indexed by session ID.
    messages: RwLock<HashMap<SessionId, Vec<Message>>>,
}

impl ConversationStore {
    /// Create a new conversation store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session.
    pub fn create_session(&self) -> GallifreyResult<SessionId> {
        let id = SessionId::new();
        let session = Session {
            id,
            started_at: Utc::now(),
            ended_at: None,
            summary: None,
            topics: Vec::new(),
            metadata: HashMap::new(),
        };

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        sessions.insert(id, session);

        Ok(id)
    }

    /// Get a session.
    pub fn get_session(&self, id: SessionId) -> GallifreyResult<Option<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(sessions.get(&id).cloned())
    }

    /// End a session.
    pub fn end_session(&self, id: SessionId) -> GallifreyResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let session = sessions
            .get_mut(&id)
            .ok_or_else(|| GallifreyError::SessionNotFound(id.to_string()))?;

        session.ended_at = Some(Utc::now());

        Ok(())
    }

    /// Add a message to a session.
    pub fn add_message(&self, message: Message) -> GallifreyResult<EntityId> {
        let id = message.id;
        let session_id = message.session_id;

        let mut messages = self
            .messages
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        messages.entry(session_id).or_default().push(message);

        Ok(id)
    }

    /// Get messages for a session.
    pub fn get_messages(&self, session_id: SessionId) -> GallifreyResult<Vec<Message>> {
        let messages = self
            .messages
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(messages.get(&session_id).cloned().unwrap_or_default())
    }

    /// Get recent messages from a session.
    pub fn get_recent_messages(&self, session_id: SessionId, limit: usize) -> GallifreyResult<Vec<Message>> {
        let messages = self
            .messages
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(messages
            .get(&session_id)
            .map(|msgs| {
                let start = msgs.len().saturating_sub(limit);
                msgs[start..].to_vec()
            })
            .unwrap_or_default())
    }

    /// Set session summary.
    pub fn set_summary(&self, id: SessionId, summary: String) -> GallifreyResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let session = sessions
            .get_mut(&id)
            .ok_or_else(|| GallifreyError::SessionNotFound(id.to_string()))?;

        session.summary = Some(summary);

        Ok(())
    }

    /// Search messages by semantic similarity (placeholder).
    pub fn semantic_search(&self, _embedding: &[f32], limit: usize) -> GallifreyResult<Vec<Message>> {
        // TODO: Implement actual vector similarity search
        let messages = self
            .messages
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(messages
            .values()
            .flat_map(|msgs| msgs.iter())
            .filter(|m| m.embedding.is_some())
            .take(limit)
            .cloned()
            .collect())
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> GallifreyResult<Vec<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(sessions.values().cloned().collect())
    }
}

impl Default for ConversationStore {
    fn default() -> Self {
        Self::new()
    }
}
