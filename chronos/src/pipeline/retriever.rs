//! Multi-source retrieval for Chronos.

use super::{ContextSource, ContextSourceType, RagConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::pipeline::analyzer::AnalyzedQuery;
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tracing::info;

/// Multi-source retriever.
pub struct Retriever {
    gallifrey: Arc<Gallifrey>,
}

impl Retriever {
    /// Create a new retriever.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Retrieve context from all configured sources.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieval fails.
    pub async fn retrieve(
        &self,
        query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        // Estimate capacity to avoid reallocations
        // knowledge: max_context_items
        // conversation: 5 (recent) + max_context_items (history)
        // system_state: temporal_refs.len()
        let estimated_capacity = if config.include_knowledge {
            config.max_context_items
        } else {
            0
        } + if config.include_conversation {
            config.max_context_items + 5
        } else {
            0
        } + if config.include_system_state {
            query.temporal_refs.len()
        } else {
            0
        };

        let mut sources = Vec::with_capacity(estimated_capacity);

        // Retrieve from each source in parallel (TODO: make truly parallel)
        if config.include_knowledge {
            sources.extend(self.retrieve_knowledge(query, config).await?);
        }

        if config.include_conversation {
            sources.extend(self.retrieve_conversation(query, config).await?);
        }

        if config.include_system_state {
            sources.extend(self.retrieve_system_state(query, config).await?);
        }

        // Sort by relevance and limit
        sources.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sources.truncate(config.max_context_items);

        Ok(sources)
    }

    /// Retrieve from knowledge graph.
    async fn retrieve_knowledge(
        &self,
        query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from knowledge graph");

        // TODO: Generate embedding for query
        let embedding: Vec<f32> = Vec::new();

        let entities = self
            .gallifrey
            .knowledge()
            .semantic_search(&embedding, config.max_context_items)
            .map_err(ChronosError::Gallifrey)?;

        Ok(entities
            .into_iter()
            .map(|e| ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: format!("{}: {:?}", e.name, e.properties),
                relevance: 0.8, // TODO: Actual relevance score
                entity_id: Some(e.id),
            })
            .collect())
    }

    /// Retrieve from conversation history.
    async fn retrieve_conversation(
        &self,
        query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from conversation history");

        // Pre-allocate for recent messages (5) and semantic search results
        let mut sources = Vec::with_capacity(5 + config.max_context_items);

        // Get recent messages from current session
        if let Some(session_id) = config.session_id {
            let messages = self
                .gallifrey
                .conversation()
                .get_recent_messages(session_id, 5)
                .map_err(ChronosError::Gallifrey)?;

            for msg in messages {
                sources.push(ContextSource {
                    source_type: ContextSourceType::Conversation,
                    content: format!("{:?}: {}", msg.role, msg.content),
                    relevance: 0.9, // Recent messages are highly relevant
                    entity_id: None,
                });
            }
        }

        // TODO: Semantic search across all conversations
        let embedding: Vec<f32> = Vec::new();
        let historical = self
            .gallifrey
            .conversation()
            .semantic_search(&embedding, config.max_context_items)
            .map_err(ChronosError::Gallifrey)?;

        for msg in historical {
            sources.push(ContextSource {
                source_type: ContextSourceType::Conversation,
                content: msg.content,
                relevance: 0.7,
                entity_id: None,
            });
        }

        Ok(sources)
    }

    /// Retrieve from system state.
    async fn retrieve_system_state(
        &self,
        query: &AnalyzedQuery,
        _config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from system state");

        let mut sources = Vec::with_capacity(query.temporal_refs.len());

        // If query has temporal references, find relevant snapshots
        for temporal_ref in &query.temporal_refs {
            if let Some(snapshot) = self
                .gallifrey
                .system_state()
                .find_snapshot_at(temporal_ref.resolved)
                .map_err(ChronosError::Gallifrey)?
            {
                sources.push(ContextSource {
                    source_type: ContextSourceType::SystemState,
                    content: format!(
                        "Snapshot '{}' at {}: {} processes, {} config items",
                        snapshot.name,
                        snapshot.timestamp,
                        snapshot.state.processes.len(),
                        snapshot.state.config.len()
                    ),
                    relevance: 0.6,
                    entity_id: None,
                });
            }
        }

        Ok(sources)
    }
}
