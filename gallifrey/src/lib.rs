//! # Tardis Gallifrey
//!
//! The temporal knowledge store for Tardis OS, integrating `GallifreyDB`.
//!
//! Gallifrey provides three specialized stores:
//! - **Knowledge Store**: Entity-relationship graph with embeddings. Used for long-term semantic memory.
//! - **Conversation Store**: Chat history with cross-session continuity. Used for context retention.
//! - **System State Store**: OS state snapshots. Used for time-travel debugging and "undo" functionality.
//!
//! ## Bi-Temporality
//!
//! All stores support bi-temporal queries, tracking two dimensions of time:
//! 1. **Valid Time**: When a fact was true in the real world.
//! 2. **Transaction Time**: When a fact was recorded in the system.
//!
//! This allows answering questions like:
//! - "What is the system state *now*?" (Current Valid, Current Transaction)
//! - "What did we *think* the system state was yesterday?" (Past Transaction, Past Valid)
//!
//! ## Getting Started
//!
//! ```rust
//! use tardis_gallifrey::Gallifrey;
//! use tardis_common::domain::Entity;
//! use tardis_common::id::EntityId;
//! use tardis_common::temporal::{BiTemporalInterval, TemporalQuery};
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Initialize the store
//! let gallifrey = Gallifrey::new();
//!
//! // 2. Create an entity representing a fact
//! let entity = Entity {
//!     id: EntityId::new(),
//!     entity_type: "Fact".to_string(),
//!     name: "The Sky".to_string(),
//!     properties: HashMap::from([
//!         ("color".to_string(), serde_json::json!("blue"))
//!     ]),
//!     embedding: None, // In real usage, this would be a vector
//!     temporal: BiTemporalInterval::now(),
//!     source: Some("User Observation".to_string()),
//! };
//!
//! // 3. Insert it into the Knowledge Store
//! let id = gallifrey.insert(entity).await?;
//!
//! // 4. Update it (creates a new version, preserving history)
//! gallifrey.update(id, serde_json::json!({"color": "dark_blue"})).await?;
//!
//! // 5. Retrieve history to see both versions
//! let history = gallifrey.get_history(id).await?;
//! assert_eq!(history.len(), 2);
//! # Ok(())
//! # }
//! ```

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

use std::sync::Arc;
use tardis_common::domain::{Change, Entity, Message, Snapshot};
use tardis_common::id::{EntityId, SessionId};
use tardis_common::temporal::TemporalQuery;
use tardis_common::traits::QueryResult;

/// The main Gallifrey database instance.
///
/// This struct acts as a facade over the specialized stores (`KnowledgeStore`, `ConversationStore`, `SystemStateStore`).
/// It provides a unified async API for the rest of the system.
#[derive(Debug)]
pub struct Gallifrey {
    knowledge: Arc<KnowledgeStore>,
    conversation: Arc<ConversationStore>,
    system_state: Arc<SystemStateStore>,
}

impl Gallifrey {
    /// Create a new Gallifrey instance with empty stores.
    #[must_use]
    pub fn new() -> Self {
        Self {
            knowledge: Arc::new(KnowledgeStore::new()),
            conversation: Arc::new(ConversationStore::new()),
            system_state: Arc::new(SystemStateStore::new()),
        }
    }

    /// Get access to the underlying knowledge store.
    #[must_use]
    pub fn knowledge(&self) -> Arc<KnowledgeStore> {
        Arc::clone(&self.knowledge)
    }

    /// Get access to the underlying conversation store.
    #[must_use]
    pub fn conversation(&self) -> Arc<ConversationStore> {
        Arc::clone(&self.conversation)
    }

    /// Get access to the underlying system state store.
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
    /// This delegates to [`KnowledgeStore::insert_entity`].
    ///
    /// # Errors
    ///
    /// Returns an error if the node cannot be inserted (e.g. storage error).
    #[allow(clippy::unused_async)]
    pub async fn insert(&self, node: Entity) -> tardis_common::Result<EntityId> {
        self.knowledge
            .insert_entity(node)
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    /// Update an existing node.
    ///
    /// This delegates to [`KnowledgeStore::update_entity`], performing a bi-temporal update.
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
    /// Returns all versions of the entity, both current and historical.
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

impl Default for Gallifrey {
    fn default() -> Self {
        Self::new()
    }
}
