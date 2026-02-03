//! Multi-source retrieval for Chronos.

use super::{ContextSource, ContextSourceType, RagConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::pipeline::analyzer::AnalyzedQuery;
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tracing::info;

/// Multi-source retriever.
#[derive(Debug)]
pub struct Retriever {
    gallifrey: Arc<Gallifrey>,
}

impl Retriever {
    /// Create a new retriever.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
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
        let mut sources = Vec::new();

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
    #[allow(clippy::unused_async)]
    async fn retrieve_knowledge(
        &self,
        _query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from knowledge graph");

        // TODO: Generate embedding for query
        let embedding: Vec<f32> = Vec::new();

        let entities = self
            .gallifrey
            .search_knowledge(&embedding, config.max_context_items)
            .await
            .map_err(ChronosError::Common)?;

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
    #[allow(clippy::unused_async)]
    async fn retrieve_conversation(
        &self,
        _query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from conversation history");

        let mut sources = Vec::new();

        // Get recent messages from current session
        if let Some(session_id) = config.session_id {
            let messages = self
                .gallifrey
                .get_recent_messages(session_id, 5)
                .await
                .map_err(ChronosError::Common)?;

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
            .search_conversation(&embedding, config.max_context_items)
            .await
            .map_err(ChronosError::Common)?;

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
    #[allow(clippy::unused_async)]
    async fn retrieve_system_state(
        &self,
        query: &AnalyzedQuery,
        _config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from system state");

        let mut sources = Vec::new();

        // If query has temporal references, find relevant snapshots
        for temporal_ref in &query.temporal_refs {
            if let Some(snapshot) = self
                .gallifrey
                .find_snapshot(temporal_ref.resolved)
                .await
                .map_err(ChronosError::Common)?
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
