//! Shared traits for Tardis subsystem interfaces.
//!
//! These traits define the contracts between subsystems, designed to support
//! both direct function calls (monolithic) and potential future IPC (microkernel).

use crate::domain::{Change, Entity, Message, Snapshot};
use crate::id::{EntityId, ModelHandle, SessionId};
use crate::llm::{InferenceParams, ModelLoadConfig};
use crate::temporal::TemporalQuery;
use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Query results from Gallifrey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result nodes
    pub nodes: Vec<Entity>,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether results were truncated
    pub truncated: bool,
}

/// Interface for LLM inference service (Vortex).
#[async_trait]
pub trait VortexService: Send + Sync + std::fmt::Debug {
    /// Load a model.
    async fn load_model(&self, path: &str, config: ModelLoadConfig) -> Result<ModelHandle>;

    /// Unload a model.
    async fn unload_model(&self, handle: ModelHandle) -> Result<()>;

    /// Run inference.
    async fn infer(
        &self,
        handle: ModelHandle,
        prompt: &str,
        params: InferenceParams,
    ) -> Result<String>;

    /// Generate embeddings.
    async fn embed(&self, handle: ModelHandle, text: &str) -> Result<Vec<f32>>;
}

/// Interface for Temporal Knowledge Graph service (Gallifrey).
#[async_trait]
pub trait GallifreyService: Send + Sync + std::fmt::Debug {
    /// Insert an entity into the knowledge graph.
    async fn insert(&self, entity: Entity) -> Result<EntityId>;

    /// Semantic search in knowledge graph.
    async fn search_knowledge(&self, embedding: &[f32], limit: usize) -> Result<Vec<Entity>>;

    /// Execute a temporal query.
    async fn query(&self, query: &str, temporal: TemporalQuery) -> Result<QueryResult>;

    /// Get recent messages from a session.
    async fn get_recent_messages(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> Result<Vec<Message>>;

    /// Semantic search in conversation history.
    async fn search_conversation(&self, embedding: &[f32], limit: usize) -> Result<Vec<Message>>;

    /// Find a system snapshot at a specific time.
    async fn find_snapshot(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Snapshot>>;

    /// Record a system change.
    async fn record_change(&self, change: Change) -> Result<()>;
}
