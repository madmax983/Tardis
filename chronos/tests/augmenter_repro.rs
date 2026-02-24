#![allow(missing_docs)]
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use tardis_chronos::pipeline::{augment, AnalyzedQuery, QueryIntent, RagConfig};

    #[test]
    fn test_augment_capacity_overflow() {
        let config = RagConfig {
            max_context_tokens: (isize::MAX as usize / 4) + 1000,
            ..RagConfig::default()
        };

        let analysis = AnalyzedQuery {
            text: "test".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        };

        // This should return an error now, not panic
        let result = augment("query", &[], &analysis, &config);
        assert!(result.is_err(), "Should return error on capacity overflow");
    }

    #[test]
    fn test_augment_excessive_limit() {
        let config = RagConfig {
            max_context_tokens: 100_001,
            ..RagConfig::default()
        };

        let analysis = AnalyzedQuery {
            text: "test".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        };

        // This should return an error because it exceeds MAX_TOKENS_LIMIT
        let result = augment("query", &[], &analysis, &config);
        assert!(result.is_err(), "Should return error on excessive limit");
        let err = result.err().unwrap().to_string();
        assert!(err.contains("exceeds limit"), "Error should mention limit");
    }
}
