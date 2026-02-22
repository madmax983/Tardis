//! Multi-source retrieval for Chronos.

use super::{ContextSource, ContextSourceType, RagConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::pipeline::analyzer::AnalyzedQuery;
use std::sync::Arc;
use tardis_gallifrey::{ConversationStore, KnowledgeStore, SystemStateStore};
use tracing::info;

/// Retrieve context from all configured sources.
///
/// # Errors
///
/// Returns an error if retrieval fails.
#[allow(clippy::unused_async)]
pub async fn retrieve(
    knowledge: &Arc<KnowledgeStore>,
    conversation: &Arc<ConversationStore>,
    system_state: &Arc<SystemStateStore>,
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
        retrieve_knowledge(knowledge, query, config, &mut sources).await?;
    }

    if config.include_conversation {
        retrieve_conversation(conversation, query, config, &mut sources).await?;
    }

    if config.include_system_state {
        retrieve_system_state(system_state, query, config, &mut sources).await?;
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
    knowledge: &Arc<KnowledgeStore>,
    _query: &AnalyzedQuery,
    config: &RagConfig,
    sources: &mut Vec<ContextSource>,
) -> ChronosResult<()> {
    info!("Retrieving from knowledge graph");

    // TODO: Generate embedding for query
    let embedding: Vec<f32> = Vec::new();

    let entities = knowledge
        .semantic_search(&embedding, config.max_context_items)
        .map_err(tardis_common::Error::from)
        .map_err(ChronosError::Common)?;

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
    conversation: &Arc<ConversationStore>,
    _query: &AnalyzedQuery,
    config: &RagConfig,
    sources: &mut Vec<ContextSource>,
) -> ChronosResult<()> {
    info!("Retrieving from conversation history");

    // Get recent messages from current session
    if let Some(session_id) = config.session_id {
        let messages = conversation
            .get_recent_messages(session_id, 5)
            .map_err(tardis_common::Error::from)
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
    let historical = conversation
        .semantic_search(&embedding, config.max_context_items)
        .map_err(tardis_common::Error::from)
        .map_err(ChronosError::Common)?;

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
    system_state: &Arc<SystemStateStore>,
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

        if let Some(snapshot) = system_state
            .find_snapshot_at(resolved)
            .map_err(tardis_common::Error::from)
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

    Ok(())
}
