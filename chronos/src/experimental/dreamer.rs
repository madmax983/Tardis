//! The Dreamer: A background process for consolidating memories.
//!
//! "We are such stuff as dreams are made on..."

use crate::error::ChronosResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tardis_common::domain::{Entity, Message};
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;
use tracing::{info, instrument};

/// Trait for catching dreams (LLM inference abstraction).
#[async_trait]
pub trait DreamCatcher: Send + Sync + std::fmt::Debug {
    /// Interpret a sequence of messages and generate an insight.
    async fn catch_dream(&self, messages: &[Message]) -> ChronosResult<String>;
}

/// The Dreamer engine.
#[derive(Debug)]
pub struct Dreamer {
    gallifrey: Arc<Gallifrey>,
    catcher: Box<dyn DreamCatcher>,
}

impl Dreamer {
    /// Create a new Dreamer.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>, catcher: Box<dyn DreamCatcher>) -> Self {
        Self { gallifrey, catcher }
    }

    /// Perform a dream cycle: Analyze recent conversations and store insights.
    ///
    /// # Errors
    ///
    /// Returns an error if fetching messages or storing insights fails.
    #[instrument(skip(self))]
    pub async fn dream(&self) -> ChronosResult<Option<EntityId>> {
        info!("Starting dream cycle...");

        // 1. Get all sessions
        let sessions = self.gallifrey.conversation().list_sessions().map_err(|e| {
            crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
        })?;

        if sessions.is_empty() {
            info!("No sessions to dream about.");
            return Ok(None);
        }

        // 2. Pick the most recent session
        // (In a real system, we might pick a random one or one with high 'perplexity')
        let mut sorted_sessions = sessions;
        sorted_sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        let target_session = &sorted_sessions[0];

        info!("Dreaming about session: {}", target_session.id);

        // 3. Fetch recent messages
        let messages = self
            .gallifrey
            .get_recent_messages(target_session.id, 10) // Last 10 messages
            .await
            .map_err(crate::error::ChronosError::Common)?;

        if messages.is_empty() {
            info!("Session has no messages.");
            return Ok(None);
        }

        // 4. Generate insight
        let insight_text = self.catcher.catch_dream(&messages).await?;
        info!("Dreamt insight: {}", insight_text);

        // 5. Store as knowledge
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Insight".to_string(),
            name: format!("Dream: Session {}", target_session.id),
            properties: {
                let mut props = HashMap::new();
                props.insert(
                    "content".to_string(),
                    serde_json::Value::String(insight_text),
                );
                props.insert(
                    "source_session".to_string(),
                    serde_json::Value::String(target_session.id.to_string()),
                );
                props
            },
            embedding: None, // TODO: Generate embedding for the insight
            temporal: tardis_common::temporal::BiTemporalInterval::now(),
            source: Some("Chronos::Dreamer".to_string()),
        };

        let id = self
            .gallifrey
            .insert(entity)
            .await
            .map_err(crate::error::ChronosError::Common)?;

        info!("Stored dream insight as entity: {}", id);
        Ok(Some(id))
    }
}

/// A Mock `DreamCatcher` for testing.
#[derive(Debug, Default)]
pub struct MockDreamCatcher;

#[async_trait]
impl DreamCatcher for MockDreamCatcher {
    async fn catch_dream(&self, messages: &[Message]) -> ChronosResult<String> {
        Ok(format!(
            "Insight from {} messages: User seems interested in time travel.",
            messages.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tardis_common::domain::{Message, Role};
    use tardis_common::id::EntityId;

    #[tokio::test]
    async fn test_dream_cycle() {
        // 1. Setup Gallifrey
        let gallifrey = Arc::new(Gallifrey::new());
        let conv_store = gallifrey.conversation();

        // 2. Create a session and add messages
        let session_id = conv_store.create_session().unwrap();

        let msg1 = Message {
            id: EntityId::new(),
            session_id,
            role: Role::User,
            content: "Tell me about the Doctor.".to_string(),
            timestamp: chrono::Utc::now(),
            embedding: None,
            entity_refs: Vec::new(),
        };
        conv_store.add_message(msg1).unwrap();

        let msg2 = Message {
            id: EntityId::new(),
            session_id,
            role: Role::Assistant,
            content: "The Doctor is a Time Lord from Gallifrey.".to_string(),
            timestamp: chrono::Utc::now(),
            embedding: None,
            entity_refs: Vec::new(),
        };
        conv_store.add_message(msg2).unwrap();

        // 3. Create Dreamer
        let dreamer = Dreamer::new(gallifrey.clone(), Box::new(MockDreamCatcher::default()));

        // 4. Dream!
        let result = dreamer.dream().await;
        assert!(result.is_ok());
        let entity_id = result.unwrap();
        assert!(entity_id.is_some());

        // 5. Verify storage
        // Since Gallifrey::query is not fully implemented/exposed for simple get,
        // we trust the insert worked if no error returned.
        // Real integration tests would verify the content.
    }
}
