//! # Echoes
//!
//! A system resonance module that finds "Echoes from the Past" based on current context.
//! It mimics "deja vu" by finding semantically similar events in the knowledge graph
//! and conversation history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_common::traits::{GallifreyService, VortexService};

/// An echo from the past.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Echo {
    /// The content of the echo (message or entity name).
    pub content: String,
    /// Where this echo came from (e.g., "Knowledge:ProjectX", "Chat:Session123").
    pub source: String,
    /// Resonance score (similarity, 0.0 to 1.0).
    pub score: f32,
    /// When the original event happened.
    pub timestamp: DateTime<Utc>,
}

/// The Echo Chamber service.
#[derive(Debug)]
pub struct EchoChamber {
    vortex: Arc<dyn VortexService>,
    gallifrey: Arc<dyn GallifreyService>,
}

impl EchoChamber {
    /// Create a new `EchoChamber`.
    #[must_use]
    pub fn new(vortex: Arc<dyn VortexService>, gallifrey: Arc<dyn GallifreyService>) -> Self {
        Self { vortex, gallifrey }
    }

    /// Listen to the current context and find echoes.
    ///
    /// # Errors
    ///
    /// Returns an error if embedding or search fails.
    pub async fn listen(&self, text: &str) -> tardis_common::Result<Vec<Echo>> {
        // 1. Get embedding for the current text
        // In a real scenario, we'd use a specific model for this.
        // For now, we assume the default model is suitable.
        let handle = self
            .vortex
            .load_model("default", tardis_common::llm::ModelLoadConfig::default())
            .await?;

        let embedding = self.vortex.embed(handle, text).await?;

        // 2. Search Knowledge Graph
        let knowledge_hits = self.gallifrey.search_knowledge(&embedding, 3).await?;

        // 3. Search Conversation History
        let chat_hits = self.gallifrey.search_conversation(&embedding, 3).await?;

        // 4. Combine and format
        let mut echoes = Vec::new();

        for entity in knowledge_hits {
            echoes.push(Echo {
                content: entity.name,
                source: format!("Knowledge:{}", entity.entity_type),
                score: 0.85, // TODO: Get actual score from vector DB
                timestamp: entity.temporal.valid_time.start,
            });
        }

        for msg in chat_hits {
            echoes.push(Echo {
                content: msg.content,
                source: "Conversation".to_string(),
                score: 0.80, // TODO: Get actual score from vector DB
                timestamp: msg.timestamp,
            });
        }

        // Sort by timestamp (newest first) for now, or score if we had it
        echoes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(echoes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tardis_common::domain::{Change, Entity, Message, Snapshot};
    use tardis_common::id::{EntityId, ModelHandle, SessionId};
    use tardis_common::llm::{InferenceParams, ModelLoadConfig};
    use tardis_common::temporal::{BiTemporalInterval, TemporalQuery};
    use tardis_common::traits::QueryResult;
    use tardis_common::Result;

    #[derive(Debug)]
    struct MockVortex;

    #[async_trait]
    impl VortexService for MockVortex {
        async fn load_model(&self, _path: &str, _config: ModelLoadConfig) -> Result<ModelHandle> {
            Ok(ModelHandle::new(0))
        }
        async fn unload_model(&self, _handle: ModelHandle) -> Result<()> {
            Ok(())
        }
        async fn infer(
            &self,
            _handle: ModelHandle,
            _prompt: &str,
            _params: InferenceParams,
        ) -> Result<String> {
            Ok("inference".to_string())
        }
        async fn embed(&self, _handle: ModelHandle, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![0.1, 0.2, 0.3])
        }
    }

    #[derive(Debug)]
    struct MockGallifrey;

    #[async_trait]
    impl GallifreyService for MockGallifrey {
        async fn insert(&self, _entity: Entity) -> Result<EntityId> {
            Ok(EntityId::new())
        }
        async fn get_history(&self, _id: EntityId) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn search_knowledge(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<Entity>> {
            Ok(vec![Entity {
                id: EntityId::new(),
                entity_type: "Fact".to_string(),
                name: "Previous System Crash".to_string(),
                properties: std::collections::HashMap::new(),
                embedding: None,
                temporal: BiTemporalInterval::now(),
                source: None,
            }])
        }
        async fn query(&self, _query: &str, _temporal: TemporalQuery) -> Result<QueryResult> {
            Ok(QueryResult {
                nodes: vec![],
                execution_time_ms: 0,
                truncated: false,
            })
        }
        async fn get_recent_messages(
            &self,
            _session_id: SessionId,
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn search_conversation(
            &self,
            _embedding: &[f32],
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![Message {
                id: EntityId::new(),
                session_id: SessionId::new(),
                role: tardis_common::domain::Role::User,
                content: "I remember when the system crashed".to_string(),
                timestamp: Utc::now(),
                embedding: None,
                entity_refs: vec![],
            }])
        }
        async fn find_snapshot(&self, _timestamp: DateTime<Utc>) -> Result<Option<Snapshot>> {
            Ok(None)
        }
        async fn get_snapshot_history(&self, _limit: usize) -> Result<Vec<Snapshot>> {
            Ok(vec![])
        }
        async fn record_change(&self, _change: Change) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_echoes() {
        let vortex = Arc::new(MockVortex);
        let gallifrey = Arc::new(MockGallifrey);
        let echo_chamber = EchoChamber::new(vortex, gallifrey);

        let echoes = echo_chamber.listen("system crash").await.unwrap();

        assert_eq!(echoes.len(), 2);
        assert!(echoes.iter().any(|e| e.content == "Previous System Crash"));
        assert!(echoes
            .iter()
            .any(|e| e.content == "I remember when the system crashed"));
    }
}
