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
//! use tardis_vortex::{Vortex, VortexLlmService, ModelLoadConfig};
//! use tardis_gallifrey::Gallifrey;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 1. Initialize services
//! let vortex = Arc::new(Vortex::new()?);
//! let handle = vortex.load_model("model.safetensors", ModelLoadConfig::default()).await?;
//! let llm = Arc::new(VortexLlmService::new(vortex, handle));
//! let gallifrey = Gallifrey::new();
//!
//! // 2. Create Chronos engine
//! let chronos = Chronos::new(
//!     llm,
//!     gallifrey.knowledge(),
//!     gallifrey.conversation(),
//!     gallifrey.system_state()
//! );
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
use tardis_common::domain::Entity;
use tardis_common::id::{EntityId, SessionId};
use tardis_common::llm::InferenceParams;
use tardis_common::traits::{
    ConversationService, KnowledgeService, LlmService, SystemStateService,
};
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
    /// System persona description (optional).
    #[cfg(feature = "nova")]
    pub persona: Option<String>,
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
            #[cfg(feature = "nova")]
            persona: None,
        }
    }
}

/// A source of context for RAG.
///
/// Represents a piece of information retrieved to answer a query.
///
/// **Note:** The `content` of a source may be truncated by the [`augment`] function
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
    llm: Arc<dyn LlmService>,
    knowledge: Arc<dyn KnowledgeService>,
    conversation: Arc<dyn ConversationService>,
    system_state: Arc<dyn SystemStateService>,
}

impl Chronos {
    /// Create a new Chronos instance.
    #[must_use]
    pub fn new(
        llm: Arc<dyn LlmService>,
        knowledge: Arc<dyn KnowledgeService>,
        conversation: Arc<dyn ConversationService>,
        system_state: Arc<dyn SystemStateService>,
    ) -> Self {
        Self {
            llm,
            knowledge,
            conversation,
            system_state,
        }
    }

    /// Execute a RAG query.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Query analysis fails.
    /// - Retrieval from Gallifrey fails.
    /// - Augmentation fails.
    /// - LLM inference fails.
    #[instrument(skip(self, config))]
    pub async fn query(&self, prompt: &str, config: RagConfig) -> ChronosResult<RagResponse> {
        info!("Processing RAG query");

        // 1. Analyze the query
        let analysis = analyzer::analyze(prompt)?;
        info!("Query analyzed: {:?}", analysis.intent);

        // 2. Retrieve relevant context
        let context = retriever::retrieve(
            &self.knowledge,
            &self.conversation,
            &self.system_state,
            &analysis,
            &config,
        )
        .await?;
        info!("Retrieved {} context items", context.len());

        // 3. Augment the prompt
        let augmented_prompt = augmenter::augment(prompt, &context, &analysis, &config)?;

        // 4. Run inference
        let params = InferenceParams::default();
        let text = match self.llm.infer(&augmented_prompt, params).await {
            Ok(t) => t,
            Err(e) => return Err(ChronosError::Common(e)),
        };

        Ok(RagResponse {
            text,
            sources: context,
            temporal_context: analysis.temporal_description,
            tokens_used: 0,
        })
    }

    /// Store a memory.
    ///
    /// Creates a new entity in the knowledge graph with the specified category.
    ///
    /// # Limitations
    ///
    /// - **Embeddings**: Currently, embeddings are not generated for new memories (TODO).
    ///   This means semantic search will not find this memory until embeddings are implemented.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity cannot be inserted into the knowledge store.
    #[allow(clippy::unused_async)]
    pub async fn remember(
        &self,
        content: &str,
        category: MemoryCategory,
    ) -> ChronosResult<EntityId> {
        info!("Storing memory: {:?}", category);

        // Create entity in knowledge graph
        let entity = Entity {
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
            .knowledge
            .insert_entity(entity)
            .await
            .map_err(ChronosError::Common)?;

        Ok(id)
    }

    /// Recall memories matching a query.
    ///
    /// Performs a semantic search in the knowledge graph.
    ///
    /// # Limitations
    ///
    /// - **Mock Implementation**: This currently passes an empty query embedding to the
    ///   knowledge service, as embedding generation is not yet implemented.
    ///
    /// # Errors
    ///
    /// Returns an error if the semantic search fails.
    #[allow(clippy::unused_async)]
    pub async fn recall(&self, query: &str, limit: usize) -> ChronosResult<Vec<ContextSource>> {
        info!("Recalling memories for: {}", &query[..query.len().min(50)]);

        // Search knowledge graph
        let results = self
            .knowledge
            .semantic_search(&[], limit)
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
