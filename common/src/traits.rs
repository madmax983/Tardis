//! Shared traits for system services.
//!
//! These traits define the interfaces for core system components, enabling
//! decoupling and easier testing/mocking.

use async_trait::async_trait;
use crate::llm::InferenceParams;
use crate::domain::{Entity, Message, Session, Snapshot, Change};
use crate::id::{EntityId, SessionId};
use crate::Result;
use std::collections::HashMap;
use std::fmt::Debug;
use std::any::Any;
use chrono::{DateTime, Utc};

/// Interface for LLM inference services.
#[async_trait]
pub trait LlmService: Send + Sync + Debug {
    /// As Any.
    fn as_any(&self) -> &dyn Any;

    /// Generate text based on a prompt.
    async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String>;

    /// Generate embeddings for text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Interface for knowledge graph storage.
#[async_trait]
pub trait KnowledgeService: Send + Sync + Debug {
    /// Insert an entity into the knowledge graph.
    async fn insert_entity(&self, entity: Entity) -> Result<EntityId>;

    /// Update an entity.
    async fn update_entity(&self, id: EntityId, updates: HashMap<String, serde_json::Value>) -> Result<()>;

    /// Get the current version of an entity.
    async fn get_entity(&self, id: EntityId) -> Result<Option<Entity>>;

    /// Get all versions of an entity.
    async fn get_entity_history(&self, id: EntityId) -> Result<Vec<Entity>>;

    /// Find entities by semantic similarity.
    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>>;
}

/// Interface for conversation history storage.
#[async_trait]
pub trait ConversationService: Send + Sync + Debug {
    /// Create a new conversation session.
    async fn create_session(&self) -> Result<SessionId>;

    /// Get a session by ID.
    async fn get_session(&self, id: SessionId) -> Result<Option<Session>>;

    /// End a session.
    async fn end_session(&self, id: SessionId) -> Result<()>;

    /// Add a message to the store.
    async fn add_message(&self, message: Message) -> Result<EntityId>;

    /// Get recent messages from a session.
    async fn get_recent_messages(&self, session_id: SessionId, limit: usize) -> Result<Vec<Message>>;

    /// Search messages by semantic similarity.
    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Message>>;
}

/// Interface for system state storage.
#[async_trait]
pub trait SystemStateService: Send + Sync + Debug {
    /// Find a snapshot at a specific time.
    async fn find_snapshot_at(&self, timestamp: DateTime<Utc>) -> Result<Option<Snapshot>>;

    /// Record a system change.
    async fn record_change(&self, change: Change) -> Result<()>;
}
