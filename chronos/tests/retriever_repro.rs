#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tardis_chronos::pipeline::{retrieve, AnalyzedQuery, QueryIntent, RagConfig};
    use tardis_gallifrey::stores::{ConversationStore, KnowledgeStore, SystemStateStore};

    #[tokio::test]
    async fn test_retrieve_excessive_limit() {
        let knowledge = Arc::new(KnowledgeStore::new());
        let conversation = Arc::new(ConversationStore::new());
        let system_state = Arc::new(SystemStateStore::new());

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
        let result = retrieve(&knowledge, &conversation, &system_state, &analysis, &config);
        assert!(result.is_err(), "Should return error on excessive limit");
        let err = result.err().unwrap().to_string();
        assert!(err.contains("exceeds limit"), "Error should mention limit");
    }
}
