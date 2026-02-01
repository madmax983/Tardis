//! Context augmentation for Chronos.

use super::{ContextSource, ContextSourceType};
use crate::error::ChronosResult;
use crate::pipeline::analyzer::AnalyzedQuery;
use chrono::Utc;

/// Context augmenter for building RAG prompts.
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
    use crate::pipeline::analyzer::{AnalyzedQuery, QueryIntent};

    fn create_dummy_analysis(intent: QueryIntent) -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test".to_string(),
            intent,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        }
    }

    #[test]
    fn test_augment_basic() {
        let augmenter = ContextAugmenter::new();
        let analysis = create_dummy_analysis(QueryIntent::Chat);
        let prompt = "Hello world";

        let result = augmenter.augment(prompt, &[], &analysis).unwrap();

        assert!(result.contains("# Tardis AI Assistant"));
        assert!(result.contains("## User Query"));
        assert!(result.contains("Hello world"));
        assert!(result.contains("## Instructions"));
        assert!(result.contains("Be helpful and concise."));
        assert!(!result.contains("## Retrieved Context"));
    }

    #[test]
    fn test_context_truncation() {
        let augmenter = ContextAugmenter::new();
        let analysis = create_dummy_analysis(QueryIntent::Chat);

        // 12000 chars / 4 = 3000 tokens
        let content_a = "A".repeat(12000);
        let content_b = "B".repeat(12000);

        let context = vec![
            ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: content_a.clone(),
                relevance: 1.0,
                entity_id: None,
            },
            ContextSource {
                source_type: ContextSourceType::Conversation,
                content: content_b.clone(),
                relevance: 0.9,
                entity_id: None,
            },
        ];

        let result = augmenter.augment("test", &context, &analysis).unwrap();

        // Should contain A
        assert!(result.contains("Knowledge 1"));
        assert!(result.contains(&content_a));

        // Should truncate B
        assert!(!result.contains(&content_b));
        assert!(result.contains("... (1 more sources truncated)"));
    }

    #[test]
    fn test_instruction_variations() {
        let augmenter = ContextAugmenter::new();

        let cases = vec![
            (QueryIntent::Recall, "accurately recalling"),
            (QueryIntent::TemporalDiff, "Compare the states"),
            (QueryIntent::SystemQuery, "accurate system state"),
            (QueryIntent::Chat, "Be helpful and concise"),
        ];

        for (intent, expected) in cases {
            let analysis = create_dummy_analysis(intent);
            let result = augmenter.augment("test", &[], &analysis).unwrap();
            assert!(
                result.contains(expected),
                "Failed for intent {:?}: expected '{}'",
                analysis.intent,
                expected
            );
        }
    }

    #[test]
    fn test_augment_system_context() {
        let augmenter = ContextAugmenter::new();
        let mut analysis = create_dummy_analysis(QueryIntent::Chat);
        analysis.temporal_description = Some("last week".to_string());

        let result = augmenter.augment("test", &[], &analysis).unwrap();

        assert!(result.contains("Query temporal context: last week"));
    }
}
