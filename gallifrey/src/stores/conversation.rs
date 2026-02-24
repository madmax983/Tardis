//! Conversation history store.
//!
//! Stores chat sessions and messages with:
//! - Cross-session continuity
//! - Semantic search across history
//! - Session summaries

pub use crate::domain::{Message, Role, Session};
use crate::error::{GallifreyError, GallifreyResult};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;
use tardis_common::{MessageId, SessionId};

/// The conversation store.
#[derive(Debug)]
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_session(&self, id: SessionId) -> GallifreyResult<Option<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(sessions.get(&id).cloned())
    }

    /// End a session.
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not found or the lock is poisoned.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn add_message(&self, message: Message) -> GallifreyResult<MessageId> {
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_messages(&self, session_id: SessionId) -> GallifreyResult<Vec<Message>> {
        let messages = self
            .messages
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(messages.get(&session_id).cloned().unwrap_or_default())
    }

    /// Get recent messages from a session.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> GallifreyResult<Vec<Message>> {
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
    ///
    /// # Errors
    ///
    /// Returns an error if the session is not found or the lock is poisoned.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn semantic_search(
        &self,
        _embedding: &[f32],
        limit: usize,
    ) -> GallifreyResult<Vec<Message>> {
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
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
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
