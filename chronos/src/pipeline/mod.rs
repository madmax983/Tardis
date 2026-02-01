//! RAG pipeline for Chronos.

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
    /// # Errors
    ///
    /// Returns an error if any stage of the pipeline fails.
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
            truncate_safe(prompt, 50),
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
            name: truncate_safe(content, 50).to_string(),
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
    /// # Errors
    ///
    /// Returns an error if retrieval fails.
    pub async fn recall(&self, query: &str, limit: usize) -> ChronosResult<Vec<ContextSource>> {
        info!("Recalling memories for: {}", truncate_safe(query, 50));

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

/// Safely truncate a string to a maximum byte length, respecting character boundaries.
fn truncate_safe(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    let mut end = max_len;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tardis_gallifrey::Gallifrey;
    use tardis_vortex::Vortex;

    #[tokio::test]
    async fn test_query_panic_on_char_boundary() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
        let chronos = Chronos::new(vortex, gallifrey);

        // 49 chars 'a', then 'é' (2 bytes).
        // 49 bytes + 2 bytes = 51 bytes.
        // min(50) = 50.
        // slice[..50] splits 'é' (byte 50 is first byte of 'é', byte 51 is second).
        // It tries to slice at 50, which is after the first byte of 'é'.
        // Wait: 'a' * 49 occupies 0..49.
        // 'é' occupies 49, 50.
        // So byte 50 IS in the middle of 'é'.
        // Yes.
        let mut prompt = "a".repeat(49);
        prompt.push('é');

        let config = RagConfig::default();
        // This should panic
        let _ = chronos.query(&prompt, config).await;
    }

    #[tokio::test]
    async fn test_remember_panic_on_char_boundary() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
        let chronos = Chronos::new(vortex, gallifrey);

        let mut content = "a".repeat(49);
        content.push('é');

        // This should panic
        let _ = chronos.remember(&content, MemoryCategory::Fact).await;
    }

    #[tokio::test]
    async fn test_recall_panic_on_char_boundary() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
        let chronos = Chronos::new(vortex, gallifrey);

        let mut query = "a".repeat(49);
        query.push('é');

        // This should panic
        let _ = chronos.recall(&query, 5).await;
    }

    #[test]
    fn test_truncate_safe_empty() {
        assert_eq!(truncate_safe("", 50), "");
    }

    #[test]
    fn test_truncate_safe_exact() {
        let s = "a".repeat(50);
        assert_eq!(truncate_safe(&s, 50), s);
    }

    #[test]
    fn test_truncate_safe_short() {
        let s = "short";
        assert_eq!(truncate_safe(s, 50), s);
    }

    #[test]
    fn test_truncate_safe_long_multibyte() {
        // "a" * 49 + "é" (2 bytes). Total 51 bytes.
        // Should truncate at 49 (before 'é') because cutting at 50 would split 'é'.
        let mut s = "a".repeat(49);
        s.push('é');
        assert_eq!(truncate_safe(&s, 50), "a".repeat(49));
    }

    #[test]
    fn test_truncate_safe_utf8() {
        // "é" is 2 bytes. "é" * 25 = 50 bytes.
        let s = "é".repeat(25);
        assert_eq!(truncate_safe(&s, 50), s);

        // "é" * 26 = 52 bytes.
        // Truncate at 50 bytes.
        let s2 = "é".repeat(26);
        assert_eq!(truncate_safe(&s2, 50), "é".repeat(25));
    }
}
