//! # Echoes
//!
//! A system resonance module that finds "Echoes from the Past" based on current context.
//! It mimics "deja vu" by finding semantically similar events in the knowledge graph
//! and conversation history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;

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
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
}

impl EchoChamber {
    /// Create a new `EchoChamber`.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
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
            .load_model("default", tardis_vortex::ModelLoadConfig::default())
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
    use tardis_common::id::{EntityId, ModelHandle, SessionId};
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::{Entity, Message};

    #[tokio::test]
    async fn test_echoes() {
        let vortex = Arc::new(Vortex::new().unwrap());
        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(0))));
        vortex.set_mock_embedding(Box::new(|_, _| Ok(vec![0.1, 0.2, 0.3])));

        let gallifrey = Arc::new(Gallifrey::new());

        // Setup Knowledge
        gallifrey
            .knowledge()
            .insert_entity(Entity {
                id: EntityId::new(),
                entity_type: "Fact".to_string(),
                name: "Previous System Crash".to_string(),
                properties: std::collections::HashMap::new(),
                embedding: Some(vec![0.1, 0.2, 0.3]),
                temporal: BiTemporalInterval::now(),
                source: None,
            })
            .unwrap();

        // Setup Conversation
        gallifrey
            .conversation()
            .add_message(Message {
                id: EntityId::new(),
                session_id: SessionId::new(),
                role: tardis_gallifrey::domain::Role::User,
                content: "I remember when the system crashed".to_string(),
                timestamp: Utc::now(),
                embedding: Some(vec![0.1, 0.2, 0.3]),
                entity_refs: vec![],
            })
            .unwrap();

        let echo_chamber = EchoChamber::new(vortex, gallifrey);

        let echoes = echo_chamber.listen("system crash").await.unwrap();

        assert_eq!(echoes.len(), 2);
        assert!(echoes.iter().any(|e| e.content == "Previous System Crash"));
        assert!(echoes
            .iter()
            .any(|e| e.content == "I remember when the system crashed"));
    }
}
