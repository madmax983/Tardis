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
//! // Initialize dependencies
//! let vortex = Arc::new(Vortex::new()?);
//! let gallifrey = Arc::new(Gallifrey::new());
//!
//! // Create Chronos engine
//! let chronos = Chronos::new(vortex, gallifrey);
//!
//! // Execute a RAG query
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

pub use analyzer::QueryAnalyzer;
pub use augmenter::ContextAugmenter;
pub use retriever::Retriever;

use crate::error::{ChronosError, ChronosResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_common::traits::{GallifreyService, VortexService};
use tardis_common::{EntityId, SessionId};
use tracing::{info, instrument};

/// Configuration for a RAG query.
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
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_context_items: 10,
            include_knowledge: true,
            include_conversation: true,
            include_system_state: false,
            session_id: None,
        }
    }
}

/// A source of context for RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source type.
    pub source_type: ContextSourceType,
    /// Content.
    pub content: String,
    /// Relevance score.
    pub relevance: f32,
    /// Entity ID if from knowledge graph.
    pub entity_id: Option<EntityId>,
}

/// Type of context source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextSourceType {
    /// Knowledge graph.
    Knowledge,
    /// Conversation history.
    Conversation,
    /// System state.
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
    #[allow(dead_code)]
    vortex: Arc<dyn VortexService>,
    gallifrey: Arc<dyn GallifreyService>,
    analyzer: QueryAnalyzer,
    retriever: Retriever,
    augmenter: ContextAugmenter,
}

impl Chronos {
    /// Create a new Chronos instance.
    #[must_use]
    pub fn new(vortex: Arc<dyn VortexService>, gallifrey: Arc<dyn GallifreyService>) -> Self {
        Self {
            vortex,
            gallifrey: Arc::clone(&gallifrey),
            analyzer: QueryAnalyzer::new(),
            retriever: Retriever::new(gallifrey),
            augmenter: ContextAugmenter::new(),
        }
    }

    /// Execute a RAG query.
    ///
    /// The query process follows this pipeline:
    /// 1. **Analysis**: The query is analyzed for intent and temporal references (e.g., "yesterday").
    /// 2. **Retrieval**: Relevant context is fetched from the knowledge graph and conversation history.
    /// 3. **Augmentation**: The context is formatted into a prompt for the LLM.
    /// 4. **Inference**: The LLM generates a response based on the augmented prompt.
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
        let analysis = self.analyzer.analyze(prompt)?;
        info!("Query analyzed: {:?}", analysis.intent);

        // 2. Retrieve relevant context
        let context = self.retriever.retrieve(&analysis, &config).await?;
        info!("Retrieved {} context items", context.len());

        // 3. Augment the prompt
        let _augmented_prompt = self.augmenter.augment(prompt, &context, &analysis)?;

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
        let entity = tardis_common::domain::Entity {
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
