//! Multi-source retrieval for Chronos.

use super::{ContextSource, ContextSourceType, RagConfig};
use crate::error::ChronosResult;
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
        // Pre-allocate to avoid resizing.
        // We expect up to max_context_items from each source plus some recent messages.
        // 3 sources * max_context_items + 5 (recent messages padding)
        let capacity = config.max_context_items * 3 + 5;
        let mut sources = Vec::with_capacity(capacity);

        // Retrieve from each source in parallel (TODO: make truly parallel)
        if config.include_knowledge {
            self.retrieve_knowledge(query, config, &mut sources).await?;
        }

        if config.include_conversation {
            self.retrieve_conversation(query, config, &mut sources)
                .await?;
        }

        if config.include_system_state {
            self.retrieve_system_state(query, config, &mut sources)
                .await?;
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
        sources: &mut Vec<ContextSource>,
    ) -> ChronosResult<()> {
        info!("Retrieving from knowledge graph");

        // TODO: Generate embedding for query
        let embedding: Vec<f32> = Vec::new();

        let entities = self
            .gallifrey
            .search_knowledge(&embedding, config.max_context_items)
            .await?;

        for e in entities {
            sources.push(ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: format!("{}: {:?}", e.name, e.properties),
                relevance: 0.8, // TODO: Actual relevance score
                entity_id: Some(e.id),
            });
        }

        Ok(())
    }

    /// Retrieve from conversation history.
    #[allow(clippy::unused_async)]
    async fn retrieve_conversation(
        &self,
        _query: &AnalyzedQuery,
        config: &RagConfig,
        sources: &mut Vec<ContextSource>,
    ) -> ChronosResult<()> {
        info!("Retrieving from conversation history");

        // Get recent messages from current session
        if let Some(session_id) = config.session_id {
            let messages = self.gallifrey.get_recent_messages(session_id, 5).await?;

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
            .await?;

        for msg in historical {
            sources.push(ContextSource {
                source_type: ContextSourceType::Conversation,
                content: msg.content,
                relevance: 0.7,
                entity_id: None,
            });
        }

        Ok(())
    }

    /// Retrieve from system state.
    #[allow(clippy::unused_async)]
    async fn retrieve_system_state(
        &self,
        query: &AnalyzedQuery,
        _config: &RagConfig,
        sources: &mut Vec<ContextSource>,
    ) -> ChronosResult<()> {
        info!("Retrieving from system state");

        // If query has temporal references, find relevant snapshots
        for temporal_ref in &query.temporal_refs {
            let Some(resolved) = temporal_ref.resolved() else {
                continue;
            };

            if let Some(snapshot) = self.gallifrey.find_snapshot(resolved).await? {
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

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::pipeline::analyzer::{AnalyzedQuery, QueryIntent};
    use std::sync::Arc;
    use tardis_gallifrey::Gallifrey;

    #[tokio::test]
    async fn test_retrieve() {
        let gallifrey = Arc::new(Gallifrey::new());
        let retriever = Retriever::new(gallifrey);

        let query = AnalyzedQuery {
            text: "test query".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: Vec::new(),
            temporal_description: None,
            entities: Vec::new(),
        };

        let config = RagConfig::default();

        let result = retriever.retrieve(&query, &config).await;
        assert!(result.is_ok());
        let sources = result.unwrap();
        // Even if empty, it should work
        assert!(sources.len() <= config.max_context_items);
    }
}
