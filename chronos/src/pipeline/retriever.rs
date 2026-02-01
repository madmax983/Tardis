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
        // Pre-allocate assuming we might get up to max_context_items from each source.
        // This reduces reallocations when merging results from different sources.
        let capacity = config.max_context_items * 3;
        let mut sources = Vec::with_capacity(capacity);

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
        _query: &AnalyzedQuery,
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
        _query: &AnalyzedQuery,
        config: &RagConfig,
    ) -> ChronosResult<Vec<ContextSource>> {
        info!("Retrieving from conversation history");

        // Pre-allocate for 5 recent messages + max_context_items historical
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

        // Pre-allocate for each temporal reference
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::analyzer::{QueryIntent, TemporalRef, TemporalRefType};
    use chrono::Utc;
    use std::collections::HashMap;
    use tardis_common::EntityId;
    use tardis_gallifrey::BiTemporalInterval;
    use tardis_gallifrey::stores::{Entity, Message, Role};

    #[tokio::test]
    async fn test_retrieve_flow() {
        // 1. Setup Gallifrey with data
        let gallifrey = Arc::new(Gallifrey::new());

        // Add an entity to knowledge store
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Concept".to_string(),
            name: "Rust Programming Language".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![0.1; 384]), // Dummy embedding
            temporal: BiTemporalInterval::now(),
            source: Some("test".to_string()),
        };
        gallifrey.knowledge().insert_entity(entity.clone()).unwrap();

        // Add a session and message to conversation store
        let session_id = gallifrey.conversation().create_session().unwrap();
        let message = Message {
            id: EntityId::new(),
            session_id,
            role: Role::User,
            content: "Tell me about Rust".to_string(),
            timestamp: Utc::now(),
            embedding: Some(vec![0.1; 384]),
            entity_refs: Vec::new(),
        };
        gallifrey.conversation().add_message(message).unwrap();

        // 2. Create Retriever
        let retriever = Retriever::new(gallifrey);

        // 3. Create Query and Config
        let query = AnalyzedQuery {
            text: "Tell me about Rust".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: vec![TemporalRef {
                text: "today".to_string(),
                resolved: Utc::now(),
                ref_type: TemporalRefType::Relative,
            }],
            temporal_description: None,
            entities: vec!["Rust".to_string()],
        };

        let config = RagConfig {
            max_context_items: 5,
            include_knowledge: true,
            include_conversation: true,
            include_system_state: true, // Enable system state to trigger that path too
            session_id: Some(session_id),
        };

        // 4. Run Retrieve
        let sources = retriever
            .retrieve(&query, &config)
            .await
            .expect("Retrieve failed");

        // 5. Verify results
        // We expect at least the knowledge entity and the message (via semantic search or recent)
        // Note: semantic search in current implementation is a dummy that returns everything with embedding
        assert!(!sources.is_empty(), "Should return some sources");

        // Verify specific source types present
        let has_knowledge = sources
            .iter()
            .any(|s| matches!(s.source_type, ContextSourceType::Knowledge));
        let has_conversation = sources
            .iter()
            .any(|s| matches!(s.source_type, ContextSourceType::Conversation));

        assert!(has_knowledge, "Should have knowledge source");
        assert!(has_conversation, "Should have conversation source");
    }
}
