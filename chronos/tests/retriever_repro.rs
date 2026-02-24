#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tardis_chronos::pipeline::{retrieve, AnalyzedQuery, QueryIntent, RagConfig};
    use tardis_common::domain::{Change, Entity, Message, Session, Snapshot};
    use tardis_common::id::{EntityId, SessionId};
    use tardis_common::traits::{ConversationService, KnowledgeService, SystemStateService};
    use tardis_common::Result;

    #[derive(Debug)]
    struct MockKnowledge;
    #[async_trait]
    impl KnowledgeService for MockKnowledge {
        async fn insert_entity(&self, _entity: Entity) -> Result<EntityId> {
            Ok(EntityId::new())
        }
        async fn update_entity(
            &self,
            _id: EntityId,
            _updates: HashMap<String, serde_json::Value>,
        ) -> Result<()> {
            Ok(())
        }
        async fn get_entity(&self, _id: EntityId) -> Result<Option<Entity>> {
            Ok(None)
        }
        async fn get_entity_history(&self, _id: EntityId) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn semantic_search(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn find_entity_by_name(&self, _name: &str) -> Result<Option<Entity>> {
            Ok(None)
        }
        async fn search_history(
            &self,
            _query: &str,
            _time: Option<DateTime<Utc>>,
            _limit: usize,
        ) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct MockConversation;
    #[async_trait]
    impl ConversationService for MockConversation {
        async fn create_session(&self) -> Result<SessionId> {
            Ok(SessionId::new())
        }
        async fn get_session(&self, _id: SessionId) -> Result<Option<Session>> {
            Ok(None)
        }
        async fn end_session(&self, _id: SessionId) -> Result<()> {
            Ok(())
        }
        async fn add_message(&self, _message: Message) -> Result<EntityId> {
            Ok(EntityId::new())
        }
        async fn get_recent_messages(
            &self,
            _session_id: SessionId,
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn semantic_search(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<Message>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct MockSystemState;
    #[async_trait]
    impl SystemStateService for MockSystemState {
        async fn find_snapshot_at(&self, _timestamp: DateTime<Utc>) -> Result<Option<Snapshot>> {
            Ok(None)
        }
        async fn record_change(&self, _change: Change) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_retrieve_excessive_limit() {
        let knowledge: Arc<dyn KnowledgeService> = Arc::new(MockKnowledge);
        let conversation: Arc<dyn ConversationService> = Arc::new(MockConversation);
        let system_state: Arc<dyn SystemStateService> = Arc::new(MockSystemState);

        let config = RagConfig {
            max_context_items: 10_001,
            ..RagConfig::default()
        };

        let analysis = AnalyzedQuery {
            text: "test".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        };

        // This should return an error because it exceeds MAX_ITEMS_LIMIT
        let result = retrieve(&knowledge, &conversation, &system_state, &analysis, &config).await;
        assert!(result.is_err(), "Should return error on excessive limit");
        let err = result.err().unwrap().to_string();
        assert!(err.contains("exceeds limit"), "Error should mention limit");
    }
}
