//! RAG pipeline for Chronos.
//!
//! This module orchestrates the Retrieval-Augmented Generation (RAG) process,
//! combining vector search from Gallifrey with LLM inference from Vortex.

mod analyzer;
mod augmenter;
mod retriever;

pub use analyzer::QueryAnalyzer;
pub use augmenter::ContextAugmenter;
pub use retriever::Retriever;

use crate::error::{ChronosError, ChronosResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_common::{EntityId, SessionId};
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// Configuration for a RAG query.
///
/// Controls how context is retrieved and used for generation.
///
/// # Examples
///
/// ```rust
/// use tardis_chronos::pipeline::RagConfig;
///
/// let config = RagConfig {
///     max_context_items: 5,
///     include_knowledge: true,
///     include_conversation: true,
///     ..Default::default()
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
///
/// Coordinates the flow of data between the user, the knowledge base, and the LLM.
///
/// # Examples
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use tardis_chronos::pipeline::{Chronos, RagConfig};
/// use tardis_vortex::Vortex;
/// use tardis_gallifrey::Gallifrey;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Initialize dependencies
/// let vortex = Arc::new(Vortex::new()?);
/// let gallifrey = Arc::new(Gallifrey::new());
///
/// // Create Chronos engine
/// let chronos = Chronos::new(vortex, gallifrey);
///
/// // Run a query
/// let response = chronos.query(
///     "What did we discuss about temporal logic?",
///     RagConfig::default()
/// ).await?;
///
/// println!("Answer: {}", response.text);
/// # Ok(())
/// # }
/// ```
pub struct Chronos {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    analyzer: QueryAnalyzer,
    retriever: Retriever,
    augmenter: ContextAugmenter,
}

impl Chronos {
    /// Create a new Chronos instance.
    #[must_use]
    pub fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
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
    /// The query process involves:
    /// 1. **Analysis**: Extracting intent and temporal references.
    /// 2. **Retrieval**: Fetching relevant context from Gallifrey.
    /// 3. **Augmentation**: Constructing a prompt with context.
    /// 4. **Generation**: Invoking Vortex to generate the response.
    ///
    /// # Errors
    ///
    /// Returns an error if any stage of the pipeline fails (e.g., database error, model error).
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
        let augmented_prompt = self.augmenter.augment(prompt, &context, &analysis)?;

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
    /// Adds a new fact or observation to the knowledge graph.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::sync::Arc;
    /// # use tardis_chronos::pipeline::{Chronos, MemoryCategory};
    /// # async fn example(chronos: Chronos) -> Result<(), Box<dyn std::error::Error>> {
    /// let id = chronos.remember(
    ///     "The user prefers dark mode.",
    ///     MemoryCategory::Preference
    /// ).await?;
    ///
    /// println!("Stored memory with ID: {}", id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub async fn remember(
        &self,
        content: &str,
        category: MemoryCategory,
    ) -> ChronosResult<EntityId> {
        info!("Storing memory: {:?}", category);

        // Create entity in knowledge graph
        let entity = tardis_gallifrey::stores::Entity {
            id: EntityId::new(),
            entity_type: format!("Memory:{:?}", category),
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
            temporal: tardis_gallifrey::BiTemporalInterval::now(),
            source: Some("user".to_string()),
        };

        let id = self
            .gallifrey
            .knowledge()
            .insert_entity(entity)
            .map_err(ChronosError::Gallifrey)?;

        Ok(id)
    }

    /// Recall memories matching a query.
    ///
    /// Performs a semantic search over stored memories.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::sync::Arc;
    /// # use tardis_chronos::pipeline::Chronos;
    /// # async fn example(chronos: Chronos) -> Result<(), Box<dyn std::error::Error>> {
    /// let memories = chronos.recall("What does the user like?", 5).await?;
    ///
    /// for memory in memories {
    ///     println!("Found: {}", memory.content);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if retrieval fails.
    pub async fn recall(&self, query: &str, limit: usize) -> ChronosResult<Vec<ContextSource>> {
        info!("Recalling memories for: {}", &query[..query.len().min(50)]);

        // Search knowledge graph
        let results = self
            .gallifrey
            .knowledge()
            .semantic_search(&[], limit)
            .map_err(ChronosError::Gallifrey)?;

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
