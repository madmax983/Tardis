//! # Tardis Gallifrey
//!
//! The temporal knowledge store for Tardis OS, integrating `GallifreyDB`.
//!
//! Gallifrey provides three specialized stores:
//! - **Knowledge Store**: Entity-relationship graph with embeddings
//! - **Conversation Store**: Chat history with cross-session continuity
//! - **System State Store**: OS state snapshots for time-travel debugging
//!
//! All stores support bi-temporal queries:
//! - **Valid time**: When the fact was true in the real world
//! - **Transaction time**: When the fact was recorded in the system

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
#[cfg(feature = "nova")]
/// Experimental features (Nova).
pub mod experimental;
pub mod query;
pub mod stores;
pub mod temporal;

// Re-export main types
pub use error::{GallifreyError, GallifreyResult};
pub use stores::{ConversationStore, KnowledgeStore, SystemStateStore};
pub use temporal::{BiTemporalInterval, TimeRange};

use async_trait::async_trait;
use std::sync::Arc;
use tardis_common::domain::{Change, Entity, Message, Snapshot};
use tardis_common::id::{EntityId, SessionId};
use tardis_common::temporal::TemporalQuery;
use tardis_common::traits::{GallifreyService, QueryResult};

/// The main Gallifrey database instance.
#[derive(Debug)]
pub struct Gallifrey {
    knowledge: Arc<KnowledgeStore>,
    conversation: Arc<ConversationStore>,
    system_state: Arc<SystemStateStore>,
}

impl Gallifrey {
    /// Create a new Gallifrey instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            knowledge: Arc::new(KnowledgeStore::new()),
            conversation: Arc::new(ConversationStore::new()),
            system_state: Arc::new(SystemStateStore::new()),
        }
    }

    /// Get access to the knowledge store.
    #[must_use]
    pub fn knowledge(&self) -> Arc<KnowledgeStore> {
        Arc::clone(&self.knowledge)
    }

    /// Get access to the conversation store.
    #[must_use]
    pub fn conversation(&self) -> Arc<ConversationStore> {
        Arc::clone(&self.conversation)
    }

    /// Get access to the system state store.
    #[must_use]
    pub fn system_state(&self) -> Arc<SystemStateStore> {
        Arc::clone(&self.system_state)
    }

    // --- Knowledge Graph ---

    /// Execute a query with optional temporal parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the query parsing or execution fails.
    #[allow(clippy::unused_async)]
    pub async fn query(
        &self,
        _query: &str,
        _temporal: TemporalQuery,
    ) -> tardis_common::Result<QueryResult> {
        // TODO: Implement actual query parsing and execution
        Ok(QueryResult {
            nodes: Vec::new(),
            execution_time_ms: 0,
            truncated: false,
        })
    }

    /// Insert a node into the knowledge graph.
    ///
    /// # Errors
    ///
    /// Returns an error if the node cannot be inserted.
    #[allow(clippy::unused_async)]
    pub async fn insert(&self, node: Entity) -> tardis_common::Result<EntityId> {
        self.knowledge
            .insert_entity(node)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Update an existing node.
    ///
    /// # Errors
    ///
    /// Returns an error if the node cannot be updated or properties are invalid.
    #[allow(clippy::unused_async)]
    pub async fn update(
        &self,
        id: EntityId,
        properties: serde_json::Value,
    ) -> tardis_common::Result<()> {
        let props: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_value(properties)
                .map_err(|e| tardis_common::Error::Internal(e.to_string()))?;

        self.knowledge
            .update_entity(id, props)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Get the history of an entity.
    ///
    /// # Errors
    ///
    /// Returns an error if history cannot be retrieved.
    #[allow(clippy::unused_async)]
    pub async fn get_history(&self, id: EntityId) -> tardis_common::Result<Vec<Entity>> {
        self.knowledge
            .get_entity_history(id)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Travel to a point in time and get a snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if time travel fails.
    #[allow(clippy::unused_async)]
    pub async fn time_travel(
        &self,
        _timestamp: chrono::DateTime<chrono::Utc>,
    ) -> tardis_common::Result<QueryResult> {
        // TODO: Implement time travel query
        Ok(QueryResult {
            nodes: Vec::new(),
            execution_time_ms: 0,
            truncated: false,
        })
    }

    /// Semantic search for entities.
    ///
    /// # Errors
    ///
    /// Returns an error if the search fails.
    #[allow(clippy::unused_async)]
    pub async fn search_knowledge(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> tardis_common::Result<Vec<Entity>> {
        self.knowledge
            .semantic_search(embedding, limit)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    // --- Conversation ---

    /// Get recent messages from a session.
    ///
    /// # Errors
    ///
    /// Returns an error if messages cannot be retrieved.
    #[allow(clippy::unused_async)]
    pub async fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> tardis_common::Result<Vec<Message>> {
        self.conversation
            .get_recent_messages(session_id, limit)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Semantic search for messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the search fails.
    #[allow(clippy::unused_async)]
    pub async fn search_conversation(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> tardis_common::Result<Vec<Message>> {
        self.conversation
            .semantic_search(embedding, limit)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    // --- System State ---

    /// Find a system snapshot at a specific time.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot cannot be found.
    #[allow(clippy::unused_async)]
    pub async fn find_snapshot(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> tardis_common::Result<Option<Snapshot>> {
        self.system_state
            .find_snapshot_at(timestamp)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Record a system change.
    ///
    /// # Errors
    ///
    /// Returns an error if the change cannot be recorded.
    #[allow(clippy::unused_async)]
    pub async fn record_change(&self, change: Change) -> tardis_common::Result<()> {
        self.system_state
            .record_change(change)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }
}

#[async_trait]
impl GallifreyService for Gallifrey {
    async fn insert(&self, entity: Entity) -> tardis_common::Result<EntityId> {
        self.insert(entity).await
    }

    async fn get_history(&self, id: EntityId) -> tardis_common::Result<Vec<Entity>> {
        self.get_history(id).await
    }

    async fn search_knowledge(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> tardis_common::Result<Vec<Entity>> {
        self.search_knowledge(embedding, limit).await
    }

    async fn query(
        &self,
        query: &str,
        temporal: TemporalQuery,
    ) -> tardis_common::Result<QueryResult> {
        self.query(query, temporal).await
    }

    async fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> tardis_common::Result<Vec<Message>> {
        self.get_recent_messages(session_id, limit).await
    }

    async fn search_conversation(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> tardis_common::Result<Vec<Message>> {
        self.search_conversation(embedding, limit).await
    }

    async fn find_snapshot(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> tardis_common::Result<Option<Snapshot>> {
        self.find_snapshot(timestamp).await
    }

    async fn record_change(&self, change: Change) -> tardis_common::Result<()> {
        self.record_change(change).await
    }
}

impl Default for Gallifrey {
    fn default() -> Self {
        Self::new()
    }
}
