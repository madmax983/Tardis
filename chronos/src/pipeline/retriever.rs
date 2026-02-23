//! Multi-source retrieval for Chronos.

use super::{ContextSource, ContextSourceType, RagConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::pipeline::analyzer::AnalyzedQuery;
use std::sync::Arc;
use tardis_common::domain::{Entity, Message, Snapshot};
use tardis_common::traits::{ConversationService, KnowledgeService, SystemStateService};
use tracing::info;

/// Retrieve context from all configured sources.
///
/// # Errors
///
/// Returns an error if retrieval fails.
pub async fn retrieve(
    knowledge: &Arc<dyn KnowledgeService>,
    conversation: &Arc<dyn ConversationService>,
    system_state: &Arc<dyn SystemStateService>,
    query: &AnalyzedQuery,
    config: &RagConfig,
) -> ChronosResult<Vec<ContextSource>> {
    // Hard limit to prevent DoS via excessive allocation
    const MAX_ITEMS_LIMIT: usize = 10_000;
    if config.max_context_items > MAX_ITEMS_LIMIT {
        return Err(ChronosError::RetrievalFailed(format!(
            "max_context_items {} exceeds limit {}",
            config.max_context_items, MAX_ITEMS_LIMIT
        )));
    }

    // Pre-allocate to avoid resizing.
    // We expect up to max_context_items from each source plus some recent messages.
    // 3 sources * max_context_items + 5 (recent messages padding)
    // Use checked arithmetic to prevent overflow
    let capacity = config
        .max_context_items
        .checked_mul(3)
        .and_then(|c| c.checked_add(5))
        .ok_or_else(|| ChronosError::RetrievalFailed("Capacity overflow".to_string()))?;

    // Bolt: Use intermediate candidate struct to avoid expensive formatting on discarded items.
    let mut candidates = Vec::with_capacity(capacity);

    // Retrieve from each source in parallel (TODO: make truly parallel)
    if config.include_knowledge {
        retrieve_knowledge(knowledge, query, config, &mut candidates).await?;
    }

    if config.include_conversation {
        retrieve_conversation(conversation, query, config, &mut candidates).await?;
    }

    if config.include_system_state {
        retrieve_system_state(system_state, query, config, &mut candidates).await?;
    }

    // Sort by relevance and limit
    // Bolt: Sorting ScoredCandidate is cheaper than sorting ContextSource (which owns Strings)
    candidates.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(config.max_context_items);

    // Bolt: Convert to ContextSource, performing expensive formatting only for winners.
    Ok(candidates
        .into_iter()
        .map(ScoredCandidate::into_context_source)
        .collect())
}

/// Intermediate candidate structure to defer string formatting.
struct ScoredCandidate {
    relevance: f32,
    data: CandidateData,
}

enum CandidateData {
    Entity(Entity),
    RecentMessage(Message),
    HistoricalMessage(Message),
    Snapshot(Snapshot),
}

impl ScoredCandidate {
    fn into_context_source(self) -> ContextSource {
        match self.data {
            CandidateData::Entity(e) => ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: format!("{}: {:?}", e.name, e.properties),
                relevance: self.relevance,
                entity_id: Some(e.id),
            },
            CandidateData::RecentMessage(msg) => ContextSource {
                source_type: ContextSourceType::Conversation,
                content: format!("{:?}: {}", msg.role, msg.content),
                relevance: self.relevance,
                entity_id: None,
            },
            CandidateData::HistoricalMessage(msg) => ContextSource {
                source_type: ContextSourceType::Conversation,
                content: msg.content,
                relevance: self.relevance,
                entity_id: None,
            },
            CandidateData::Snapshot(s) => ContextSource {
                source_type: ContextSourceType::SystemState,
                content: format!(
                    "Snapshot '{}' at {}: {} processes, {} config items",
                    s.name,
                    s.timestamp,
                    s.state.processes.len(),
                    s.state.config.len()
                ),
                relevance: self.relevance,
                entity_id: None,
            },
        }
    }
}

/// Retrieve from knowledge graph.
#[allow(clippy::unused_async)]
async fn retrieve_knowledge(
    knowledge: &Arc<dyn KnowledgeService>,
    _query: &AnalyzedQuery,
    config: &RagConfig,
    candidates: &mut Vec<ScoredCandidate>,
) -> ChronosResult<()> {
    info!("Retrieving from knowledge graph");

    // TODO: Generate embedding for query
    let embedding: Vec<f32> = Vec::new();

    let entities = knowledge
        .semantic_search(&embedding, config.max_context_items)
        .await
        .map_err(ChronosError::Common)?;

    for e in entities {
        candidates.push(ScoredCandidate {
            relevance: 0.8, // TODO: Actual relevance score
            data: CandidateData::Entity(e),
        });
    }

    Ok(())
}

/// Retrieve from conversation history.
#[allow(clippy::unused_async)]
async fn retrieve_conversation(
    conversation: &Arc<dyn ConversationService>,
    _query: &AnalyzedQuery,
    config: &RagConfig,
    candidates: &mut Vec<ScoredCandidate>,
) -> ChronosResult<()> {
    info!("Retrieving from conversation history");

    // Get recent messages from current session
    if let Some(session_id) = config.session_id {
        let messages = conversation
            .get_recent_messages(session_id, 5)
            .await
            .map_err(ChronosError::Common)?;

        for msg in messages {
            candidates.push(ScoredCandidate {
                relevance: 0.9, // Recent messages are highly relevant
                data: CandidateData::RecentMessage(msg),
            });
        }
    }

    // TODO: Semantic search across all conversations
    let embedding: Vec<f32> = Vec::new();
    let historical = conversation
        .semantic_search(&embedding, config.max_context_items)
        .await
        .map_err(ChronosError::Common)?;

    for msg in historical {
        candidates.push(ScoredCandidate {
            relevance: 0.7,
            data: CandidateData::HistoricalMessage(msg),
        });
    }

    Ok(())
}

/// Retrieve from system state.
#[allow(clippy::unused_async)]
async fn retrieve_system_state(
    system_state: &Arc<dyn SystemStateService>,
    query: &AnalyzedQuery,
    _config: &RagConfig,
    candidates: &mut Vec<ScoredCandidate>,
) -> ChronosResult<()> {
    info!("Retrieving from system state");

    // If query has temporal references, find relevant snapshots
    for temporal_ref in &query.temporal_refs {
        let Some(resolved) = temporal_ref.resolved() else {
            continue;
        };

        if let Some(snapshot) = system_state
            .find_snapshot_at(resolved)
            .await
            .map_err(ChronosError::Common)?
        {
            candidates.push(ScoredCandidate {
                relevance: 0.6,
                data: CandidateData::Snapshot(snapshot),
            });
        }
    }

    Ok(())
}
