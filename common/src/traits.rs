//! Shared traits for Tardis subsystem interfaces.
//!
//! These traits define the contracts between subsystems, designed to support
//! both direct function calls (monolithic) and potential future IPC (microkernel).

use crate::error::Result;
use crate::id::{EntityId, ModelHandle, SessionId};
use crate::temporal::{BiTemporalInterval, TemporalQuery};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================================
// Vortex (LLM) Traits
// ============================================================================

/// Configuration for loading a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelLoadConfig {
    /// Device to load on ("cpu", "cuda:0", etc.)
    pub device: Option<String>,
    /// Quantization to use (if model supports it)
    pub quantization: Option<String>,
    /// Maximum context length
    pub max_context_length: Option<usize>,
    /// Use memory mapping for weights
    pub use_mmap: bool,
}

/// Parameters for inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParams {
    /// Sampling temperature (0.0 = deterministic)
    pub temperature: f32,
    /// Top-p sampling
    pub top_p: f32,
    /// Top-k sampling
    pub top_k: usize,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 2048,
            stop_sequences: Vec::new(),
        }
    }
}

/// Information about a loaded or available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model architecture (llama, mistral, phi, etc.)
    pub architecture: String,
    /// Number of parameters
    pub parameters: u64,
    /// Maximum context length
    pub context_length: usize,
    /// Whether the model is currently loaded
    pub loaded: bool,
    /// Memory usage if loaded
    pub memory_bytes: Option<u64>,
}

/// The Vortex LLM service interface.
#[async_trait]
pub trait VortexService: Send + Sync {
    /// Load a model from disk.
    async fn load_model(&self, path: &str, config: ModelLoadConfig) -> Result<ModelHandle>;

    /// Unload a model.
    async fn unload_model(&self, handle: ModelHandle) -> Result<()>;

    /// Run inference and return generated text.
    async fn infer(&self, handle: ModelHandle, prompt: &str, params: InferenceParams)
        -> Result<String>;

    /// Generate embeddings for text.
    async fn embed(&self, handle: ModelHandle, text: &str) -> Result<Vec<f32>>;

    /// List available and loaded models.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Get information about a specific model.
    async fn model_info(&self, handle: ModelHandle) -> Result<ModelInfo>;
}

// ============================================================================
// Gallifrey (Database) Traits
// ============================================================================

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier
    pub id: EntityId,
    /// Node type/label
    pub node_type: String,
    /// Node properties
    pub properties: serde_json::Value,
    /// Embedding vector (if computed)
    pub embedding: Option<Vec<f32>>,
    /// Temporal metadata
    pub temporal: BiTemporalInterval,
}

/// Query results from Gallifrey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Result nodes
    pub nodes: Vec<GraphNode>,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether results were truncated
    pub truncated: bool,
}

/// The Gallifrey temporal database service interface.
#[async_trait]
pub trait GallifreyService: Send + Sync {
    /// Execute a query with optional temporal parameters.
    async fn query(&self, query: &str, temporal: TemporalQuery) -> Result<QueryResult>;

    /// Insert a node into the knowledge graph.
    async fn insert(&self, node: GraphNode) -> Result<EntityId>;

    /// Update an existing node.
    async fn update(&self, id: EntityId, properties: serde_json::Value) -> Result<()>;

    /// Get the history of an entity.
    async fn get_history(&self, id: EntityId) -> Result<Vec<GraphNode>>;

    /// Travel to a point in time and get a snapshot.
    async fn time_travel(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Result<QueryResult>;
}

// ============================================================================
// Chronos (RAG) Traits
// ============================================================================

/// Configuration for a RAG query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RagConfig {
    /// Maximum number of context items to retrieve
    pub max_context_items: usize,
    /// Temporal constraints for retrieval
    pub temporal: TemporalQuery,
    /// Whether to include conversation history
    pub include_conversation: bool,
    /// Whether to include system state
    pub include_system_state: bool,
}

/// A RAG response with context and sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// Generated response text
    pub text: String,
    /// Sources used for context
    pub sources: Vec<ContextSource>,
    /// Temporal grounding information
    pub temporal_context: Option<String>,
}

/// A source used for RAG context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source type (knowledge, conversation, system)
    pub source_type: String,
    /// Content excerpt
    pub content: String,
    /// Relevance score
    pub relevance: f32,
    /// When this information was valid
    pub valid_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Category for stored memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryCategory {
    /// General knowledge
    Knowledge,
    /// User preference
    Preference,
    /// Factual information
    Fact,
}

/// The Chronos RAG service interface.
#[async_trait]
pub trait ChronosService: Send + Sync {
    /// Execute a RAG query.
    async fn query(&self, prompt: &str, config: RagConfig) -> Result<RagResponse>;

    /// Store a memory.
    async fn remember(&self, content: &str, category: MemoryCategory) -> Result<EntityId>;

    /// Recall memories matching a query.
    async fn recall(&self, query: &str, temporal: TemporalQuery) -> Result<Vec<ContextSource>>;

    /// Summarize a conversation session.
    async fn summarize(&self, session_id: SessionId) -> Result<String>;
}
