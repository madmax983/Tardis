//! Shared traits for system services.
//!
//! These traits define the core architectural boundaries of Tardis OS. By defining services
//! as traits, we enable:
//!
//! 1.  **Decoupling**: Consumers (like `Chronos`) depend on abstract interfaces, not concrete implementations.
//! 2.  **Testability**: We can easily swap in mock implementations for unit testing.
//! 3.  **Flexibility**: We can change the underlying storage or inference engine without breaking consumers.
//!
//! # The Big Four Services
//!
//! *   [`LlmService`]: The brain. Handles text generation and embedding.
//! *   [`KnowledgeService`]: The long-term memory. Stores facts and relationships.
//! *   [`ConversationService`]: The short-term memory. Manages chat history.
//! *   [`SystemStateService`]: The nervous system. Tracks OS state changes.
//!
//! # `async_trait`
//!
//! You'll notice all traits are annotated with `#[async_trait]`. This is because Rust
//! currently does not support async functions in traits natively (without boxing).
//! This macro boxes the returned futures, making the traits object-safe (`dyn Trait`).
//!
//! # Example: Mocking for Tests
//!
//! ```rust
//! use tardis_common::traits::LlmService;
//! use tardis_common::llm::InferenceParams;
//! use tardis_common::Result;
//! use async_trait::async_trait;
//! use std::any::Any;
//!
//! #[derive(Debug)]
//! struct MockLlm;
//!
//! #[async_trait]
//! impl LlmService for MockLlm {
//!     async fn infer(&self, prompt: &str, _params: InferenceParams) -> Result<String> {
//!         Ok(format!("Mock response to: {}", prompt))
//!     }
//!
//!     async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
//!         Ok(vec![0.0; 384])
//!     }
//!
//!     fn as_any(&self) -> &dyn Any {
//!         self
//!     }
//! }
//! ```

use crate::domain::{Change, Entity, Message, Session, Snapshot};
use crate::id::{EntityId, SessionId};
use crate::llm::InferenceParams;
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

/// Interface for LLM inference services.
///
/// This trait abstracts over different LLM backends (e.g., local `Vortex`, remote APIs).
/// It provides methods for text generation (`infer`) and vector embedding (`embed`).
#[async_trait]
pub trait LlmService: Send + Sync + Debug {
    /// Generate text based on a prompt.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input text or prompt for the model.
    /// * `params` - Configuration for generation (temperature, max tokens, etc.).
    async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String>;

    /// Generate embeddings for text.
    ///
    /// Used for semantic search. Returns a vector of floats representing the semantic meaning.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Downcast to concrete type.
    ///
    /// Useful when you need access to backend-specific methods not exposed by the trait.
    fn as_any(&self) -> &dyn Any;
}

/// Interface for knowledge graph storage.
///
/// Handles the "Long-Term Memory" of the system.
/// Stores entities (facts) and relationships in a bi-temporal graph structure.
#[async_trait]
pub trait KnowledgeService: Send + Sync + Debug {
    /// Insert an entity into the knowledge graph.
    ///
    /// Returns the assigned `EntityId`.
    async fn insert_entity(&self, entity: Entity) -> Result<EntityId>;

    /// Update an entity.
    ///
    /// This performs a bi-temporal update (preserving history).
    /// `updates` is a map of property names to new values.
    async fn update_entity(
        &self,
        id: EntityId,
        updates: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Get the current version of an entity.
    async fn get_entity(&self, id: EntityId) -> Result<Option<Entity>>;

    /// Get all versions of an entity (history).
    ///
    /// Returns a list of all historical states of the entity, sorted by time.
    async fn get_entity_history(&self, id: EntityId) -> Result<Vec<Entity>>;

    /// Find entities by semantic similarity.
    ///
    /// Uses vector search on the `embedding` field.
    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>>;

    /// Find an entity by name.
    async fn find_entity_by_name(&self, name: &str) -> Result<Option<Entity>>;

    /// Search entity history.
    async fn search_history(
        &self,
        query: &str,
        time: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<Entity>>;
}

/// Interface for conversation history storage.
///
/// Handles the "Short-Term Memory" of the system (context window).
/// Groups messages into sessions.
#[async_trait]
pub trait ConversationService: Send + Sync + Debug {
    /// Create a new conversation session.
    async fn create_session(&self) -> Result<SessionId>;

    /// Get a session by ID.
    async fn get_session(&self, id: SessionId) -> Result<Option<Session>>;

    /// End a session.
    ///
    /// Typically triggers summarization and archival.
    async fn end_session(&self, id: SessionId) -> Result<()>;

    /// Add a message to the store.
    async fn add_message(&self, message: Message) -> Result<EntityId>;

    /// Get recent messages from a session.
    ///
    /// Useful for rebuilding the context window for the LLM.
    async fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> Result<Vec<Message>>;

    /// Search messages by semantic similarity.
    async fn semantic_search(&self, embedding: &[f32], limit: usize) -> Result<Vec<Message>>;
}

/// Interface for system state storage.
///
/// Handles the "Nervous System" or "Audit Log".
/// Records changes to the OS environment and allows time-travel debugging.
#[async_trait]
pub trait SystemStateService: Send + Sync + Debug {
    /// Find a snapshot at a specific time.
    ///
    /// Used to reconstruct the system state for a past event.
    async fn find_snapshot_at(&self, timestamp: DateTime<Utc>) -> Result<Option<Snapshot>>;

    /// Record a system change.
    async fn record_change(&self, change: Change) -> Result<()>;
}
