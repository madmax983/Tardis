//! RAG pipeline for Chronos.
//!
//! This module provides the core orchestration logic for Retrieval-Augmented Generation.
//! It coordinates between the LLM (Vortex) and the Temporal Knowledge Graph (Gallifrey).
//!
//! # Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use tardis_chronos::{Chronos, RagConfig};
//! use tardis_vortex::Vortex;
//! use tardis_gallifrey::Gallifrey;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Initialize dependencies
//! // In a real app, these would be shared instances
//! let vortex = Arc::new(Vortex::new()?);
//! let gallifrey = Arc::new(Gallifrey::new());
//!
//! // 2. Create Chronos engine
//! let chronos = Chronos::new(vortex, gallifrey);
//!
//! // 3. Execute a RAG query
//! let response = chronos.query(
//!     "What happened in the last session?",
//!     RagConfig::default()
//! ).await?;
//!
//! println!("Response: {}", response.text);
//! # Ok(())
//! # }
//! ```

mod analyzer;
mod augmenter;
mod retriever;

pub use analyzer::{analyze, analyze_at, AnalyzedQuery, QueryIntent};
pub use augmenter::augment;
pub use retriever::retrieve;

use crate::error::{ChronosError, ChronosResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_common::{EntityId, SessionId};
use tardis_gallifrey::Gallifrey;
#[cfg(feature = "telemetry")]
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// Configuration for a RAG query.
///
/// Controls how context is retrieved and filtered.
///
/// # Token Budget & Truncation
///
/// Chronos enforces a strict token budget (default: 4096 tokens) to prevent context window overflow.
/// Retrieved context items are prioritized by relevance. If the budget is exceeded:
/// 1. Lower-relevance items are dropped entirely.
/// 2. The last included item may be silently truncated to fit the remaining space.
///
/// # Examples
///
/// ```rust
/// use tardis_chronos::RagConfig;
///
/// let config = RagConfig {
///     max_context_items: 5,
///     include_knowledge: true,
///     include_conversation: false,
///     ..RagConfig::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Maximum number of context items to retrieve.
    pub max_context_items: usize,
    /// Include knowledge graph results.
    pub include_knowledge: bool,
    /// Include conversation history.
    pub include_conversation: bool,
    /// Include system state.
    pub include_system_state: bool,
    /// Current session ID.
    pub session_id: Option<SessionId>,
    /// Maximum number of tokens for context.
    pub max_context_tokens: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_context_items: 10,
            include_knowledge: true,
            include_conversation: true,
            include_system_state: false,
            session_id: None,
            max_context_tokens: 4096,
        }
    }
}

/// A source of context for RAG.
///
/// Represents a piece of information retrieved to answer a query.
///
/// **Note:** The `content` of a source may be truncated by the [`ContextAugmenter`]
/// if it exceeds the remaining token budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source type (where this information came from).
    pub source_type: ContextSourceType,
    /// The actual text content.
    pub content: String,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f32,
    /// Entity ID if from knowledge graph.
    pub entity_id: Option<EntityId>,
}

/// Type of context source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextSourceType {
    /// Knowledge graph (long-term memory).
    Knowledge,
    /// Conversation history (short-term memory).
    Conversation,
    /// System state (real-time data).
    SystemState,
}

/// Response from a RAG query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// Generated text.
    pub text: String,
    /// Sources used.
    pub sources: Vec<ContextSource>,
    /// Temporal context description.
    pub temporal_context: Option<String>,
    /// Token usage.
    pub tokens_used: usize,
}

/// The main Chronos RAG engine.
#[derive(Debug)]
pub struct Chronos {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    #[cfg(feature = "telemetry")]
    telemetry: Option<Arc<TelemetryStore>>,
}

impl Chronos {
    /// Create a new Chronos instance.
    #[must_use]
    pub fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            vortex,
            gallifrey,
            #[cfg(feature = "telemetry")]
            telemetry: None,
        }
    }

    /// Attach a telemetry store to Chronos.
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: Arc<TelemetryStore>) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Get access to the underlying Vortex engine.
    #[must_use]
    pub fn vortex(&self) -> Arc<Vortex> {
        Arc::clone(&self.vortex)
    }

    /// Execute a RAG query.
    ///
    /// The query process follows this pipeline:
    /// 1. **Analysis**: The query is analyzed for intent and temporal references (e.g., "yesterday").
    /// 2. **Retrieval**: Relevant context is fetched from the knowledge graph and conversation history.
    /// 3. **Augmentation**: The context is formatted into a prompt for the LLM.
    /// 4. **Inference**: The LLM generates a response based on the augmented prompt.
    ///
    /// # ⚠️ Mock Implementation
    ///
    /// Currently, the inference step is **mocked**. It will return a static string
    /// indicating what *would* have been sent to the LLM, along with the retrieved context items.
    ///
    /// **Why?** The `vortex` crate integration is in progress. This allows testing the
    /// orchestration pipeline (analysis -> retrieval -> augmentation) without loading full LLM weights.
    ///
    /// # Errors
    ///
    /// Returns an error if any stage of the pipeline fails, such as:
    /// - Vector database connection errors.
    /// - LLM inference failures.
    /// - Tokenization limits exceeded.
    #[instrument(skip(self, config))]
    pub async fn query(&self, prompt: &str, config: RagConfig) -> ChronosResult<RagResponse> {
        info!("Processing RAG query");

        // 1. Analyze the query
        let analysis = analyzer::analyze(prompt)?;
        info!("Query analyzed: {:?}", analysis.intent);

        // 2. Retrieve relevant context
        let context = retriever::retrieve(&self.gallifrey, &analysis, &config).await?;
        info!("Retrieved {} context items", context.len());

        // 3. Augment the prompt
        let _augmented_prompt = augmenter::augment(prompt, &context, &analysis, &config)?;

        // 4. Run inference
        // TODO: Use actual model handle
        let text = format!(
            "[Chronos] RAG response for: {}... (with {} context items)",
            &prompt[..prompt.len().min(50)],
            context.len()
        );

        Ok(RagResponse {
            text,
            sources: context,
            temporal_context: analysis.temporal_description,
            tokens_used: 0,
        })
    }

    /// Store a memory.
    ///
    /// This is a convenience wrapper around `Gallifrey::insert`. It creates an `Entity`
    /// representing the memory and stores it in the Knowledge Graph.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    #[allow(clippy::unused_async)]
    pub async fn remember(
        &self,
        content: &str,
        category: MemoryCategory,
    ) -> ChronosResult<EntityId> {
        info!("Storing memory: {:?}", category);

        // Create entity in knowledge graph
        let entity = tardis_gallifrey::domain::Entity {
            id: EntityId::new(),
            entity_type: format!("Memory:{category:?}"),
            name: content[..content.len().min(50)].to_string(),
            properties: {
                let mut props = std::collections::HashMap::new();
                props.insert(
                    "content".to_string(),
                    serde_json::Value::String(content.to_string()),
                );
                props
            },
            embedding: None, // TODO: Generate embedding
            temporal: tardis_common::temporal::BiTemporalInterval::now(),
            source: Some("user".to_string()),
        };

        let id = self
            .gallifrey
            .insert(entity)
            .await
            .map_err(ChronosError::Common)?;

        Ok(id)
    }

    /// Recall memories matching a query.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieval fails.
    #[allow(clippy::unused_async)]
    pub async fn recall(&self, query: &str, limit: usize) -> ChronosResult<Vec<ContextSource>> {
        info!("Recalling memories for: {}", &query[..query.len().min(50)]);

        // Search knowledge graph
        let results = self
            .gallifrey
            .search_knowledge(&[], limit)
            .await
            .map_err(ChronosError::Common)?;

        Ok(results
            .into_iter()
            .map(|e| ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: e.name,
                relevance: 1.0,
                entity_id: Some(e.id),
            })
            .collect())
    }
}

/// Category for stored memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCategory {
    /// General knowledge.
    Knowledge,
    /// User preference.
    Preference,
    /// Factual information.
    Fact,
}
