//! Context augmentation for Chronos.

use super::{ContextSource, ContextSourceType};
use crate::error::ChronosResult;
use crate::pipeline::analyzer::AnalyzedQuery;
use chrono::Utc;

/// Context augmenter for building RAG prompts.
#[derive(Debug)]
pub struct ContextAugmenter {
    /// Maximum tokens for context.
    max_context_tokens: usize,
}

impl ContextAugmenter {
    /// Create a new context augmenter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_context_tokens: 4096,
        }
    }

    /// Augment a prompt with retrieved context.
    ///
    /// # Errors
    ///
    /// Returns an error if augmentation fails.
    pub fn augment(
        &self,
        prompt: &str,
        context: &[ContextSource],
        analysis: &AnalyzedQuery,
    ) -> ChronosResult<String> {
        let mut augmented = String::new();

        // System context
        augmented.push_str(&self.build_system_context(analysis));

        // Retrieved context
        if !context.is_empty() {
            augmented.push_str("\n## Retrieved Context\n\n");
            augmented.push_str(&self.format_context(context));
        }

        // User query
        augmented.push_str("\n## User Query\n\n");
        augmented.push_str(prompt);

        // Instructions
        augmented.push_str("\n\n## Instructions\n\n");
        augmented.push_str(&self.build_instructions(analysis));

        Ok(augmented)
    }

    /// Build system context header.
    fn build_system_context(&self, analysis: &AnalyzedQuery) -> String {
        let mut ctx = String::new();

        ctx.push_str("# Tardis AI Assistant\n\n");
        ctx.push_str(&format!(
            "Current time: {}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(ref temporal) = analysis.temporal_description {
            ctx.push_str(&format!("Query temporal context: {}\n", temporal));
        }

        ctx
    }

    /// Format retrieved context.
    fn format_context(&self, context: &[ContextSource]) -> String {
        let mut formatted = String::new();
        let mut token_estimate = 0;

        for (i, source) in context.iter().enumerate() {
            // Rough token estimate (4 chars per token)
            let source_tokens = source.content.len() / 4;
            if token_estimate + source_tokens > self.max_context_tokens {
                formatted.push_str(&format!(
                    "\n... ({} more sources truncated)\n",
                    context.len() - i
                ));
                break;
            }

            let source_type = match source.source_type {
                ContextSourceType::Knowledge => "Knowledge",
                ContextSourceType::Conversation => "Conversation",
                ContextSourceType::SystemState => "System State",
            };

            formatted.push_str(&format!(
                "### {} {} (relevance: {:.2})\n{}\n\n",
                source_type,
                i + 1,
                source.relevance,
                source.content
            ));

            token_estimate += source_tokens;
        }

        formatted
    }

    /// Build response instructions based on query analysis.
    fn build_instructions(&self, analysis: &AnalyzedQuery) -> String {
        let mut instructions = String::new();

        instructions.push_str("Respond based on the context provided. ");

        match analysis.intent {
            super::analyzer::QueryIntent::Recall => {
                instructions.push_str("Focus on accurately recalling the requested information. ");
                instructions.push_str("Cite specific sources and times when available. ");
            }
            super::analyzer::QueryIntent::TemporalDiff => {
                instructions.push_str("Compare the states across the referenced time periods. ");
                instructions.push_str("Highlight what changed and when. ");
            }
            super::analyzer::QueryIntent::SystemQuery => {
                instructions.push_str("Provide accurate system state information. ");
                instructions.push_str("Include relevant timestamps and snapshots. ");
            }
            _ => {
                instructions.push_str("Be helpful and concise. ");
            }
        }

        instructions.push_str("If information comes from a specific time, mention when. ");
        instructions.push_str("If you're uncertain about something, say so.");

        instructions
    }
}

impl Default for ContextAugmenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::analyzer::QueryIntent;

    fn create_dummy_analysis(intent: QueryIntent) -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test query".to_string(),
            intent,
            temporal_refs: Vec::new(),
            temporal_description: None,
            entities: Vec::new(),
        }
    }

    #[test]
    fn should_augment_prompt_with_context() {
        let augmenter = ContextAugmenter::new();
        let context = vec![
            ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: "Knowledge 1".to_string(),
                relevance: 0.9,
                entity_id: None,
            },
            ContextSource {
                source_type: ContextSourceType::Conversation,
                content: "Conversation 1".to_string(),
                relevance: 0.8,
                entity_id: None,
            },
        ];
        let analysis = create_dummy_analysis(QueryIntent::Question);

        let result = augmenter
            .augment("User Query", &context, &analysis)
            .unwrap();

        assert!(result.contains("### Knowledge 1"));
        assert!(result.contains("Knowledge 1"));
        assert!(result.contains("### Conversation 2"));
        assert!(result.contains("Conversation 1"));
        assert!(result.contains("## User Query"));
        assert!(result.contains("User Query"));
        assert!(result.contains("## Instructions"));
    }

    #[test]
    fn should_handle_empty_context() {
        let augmenter = ContextAugmenter::new();
        let context = vec![];
        let analysis = create_dummy_analysis(QueryIntent::Question);

        let result = augmenter
            .augment("User Query", &context, &analysis)
            .unwrap();

        assert!(!result.contains("## Retrieved Context"));
        assert!(result.contains("## User Query"));
    }

    #[test]
    fn should_truncate_context_when_exceeding_token_limit() {
        let augmenter = ContextAugmenter::new();
        let analysis = create_dummy_analysis(QueryIntent::Question);

        // Source 1: 8200 chars -> 2050 tokens. estimate = 2050.
        // Source 2: 8200 chars -> 2050 tokens. estimate = 2050 + 2050 = 4100 > 4096.
        // So Source 2 should be truncated?

        let huge_content = "a".repeat(8200);
        let context_huge = vec![
            ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: huge_content.clone(),
                relevance: 0.9,
                entity_id: None,
            },
            ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: huge_content.clone(),
                relevance: 0.8,
                entity_id: None,
            },
        ];

        let result_huge = augmenter
            .augment("User Query", &context_huge, &analysis)
            .unwrap();

        // First source should be present
        assert!(result_huge.contains("### Knowledge 1"));

        // Second source should trigger truncation because 2050 + 2050 = 4100 > 4096
        assert!(result_huge.contains("... (1 more sources truncated)"));
        assert!(!result_huge.contains("### Knowledge 2"));
    }

    #[test]
    fn should_tailor_instructions_to_intent() {
        let augmenter = ContextAugmenter::new();
        let context = vec![];

        // Recall
        let analysis = create_dummy_analysis(QueryIntent::Recall);
        let result = augmenter.augment("Q", &context, &analysis).unwrap();
        assert!(result.contains("Focus on accurately recalling"));

        // TemporalDiff
        let analysis = create_dummy_analysis(QueryIntent::TemporalDiff);
        let result = augmenter.augment("Q", &context, &analysis).unwrap();
        assert!(result.contains("Compare the states"));

        // SystemQuery
        let analysis = create_dummy_analysis(QueryIntent::SystemQuery);
        let result = augmenter.augment("Q", &context, &analysis).unwrap();
        assert!(result.contains("Provide accurate system state"));
    }

    #[test]
    fn should_include_temporal_context_in_system_prompt() {
        let augmenter = ContextAugmenter::new();
        let context = vec![];
        let mut analysis = create_dummy_analysis(QueryIntent::Question);
        analysis.temporal_description = Some("yesterday".to_string());

        let result = augmenter.augment("Q", &context, &analysis).unwrap();
        assert!(result.contains("Query temporal context: yesterday"));
    }
}
