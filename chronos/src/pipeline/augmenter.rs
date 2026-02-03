//! Context augmentation for Chronos.

use super::{ContextSource, ContextSourceType};
use crate::error::ChronosResult;
use crate::pipeline::analyzer::AnalyzedQuery;
use chrono::Utc;
use std::fmt::Write as _;

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
        Self::build_system_context(analysis, &mut augmented);

        // Retrieved context
        if !context.is_empty() {
            augmented.push_str("\n## Retrieved Context\n\n");
            self.format_context(context, &mut augmented);
        }

        // User query
        augmented.push_str("\n## User Query\n\n");
        augmented.push_str(prompt);

        // Instructions
        augmented.push_str("\n\n## Instructions\n\n");
        Self::build_instructions(analysis, &mut augmented);

        Ok(augmented)
    }

    /// Build system context header.
    fn build_system_context(analysis: &AnalyzedQuery, out: &mut String) {
        out.push_str("# Tardis AI Assistant\n\n");
        let _ = writeln!(
            out,
            "Current time: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(ref temporal) = analysis.temporal_description {
            let _ = writeln!(out, "Query temporal context: {temporal}");
        }
    }

    /// Format retrieved context.
    fn format_context(&self, context: &[ContextSource], out: &mut String) {
        let mut token_estimate = 0;

        for (i, source) in context.iter().enumerate() {
            // Rough token estimate (4 chars per token)
            let source_tokens = source.content.len() / 4;
            if token_estimate + source_tokens > self.max_context_tokens {
                let _ = writeln!(
                    out,
                    "\n... ({} more sources truncated)",
                    context.len() - i
                );
                break;
            }

            let source_type = match source.source_type {
                ContextSourceType::Knowledge => "Knowledge",
                ContextSourceType::Conversation => "Conversation",
                ContextSourceType::SystemState => "System State",
            };

            let _ = writeln!(
                out,
                "### {} {} (relevance: {:.2})\n{}\n",
                source_type,
                i + 1,
                source.relevance,
                source.content
            );

            token_estimate += source_tokens;
        }
    }

    /// Build response instructions based on query analysis.
    fn build_instructions(analysis: &AnalyzedQuery, out: &mut String) {
        out.push_str("Respond based on the context provided. ");

        match analysis.intent {
            super::analyzer::QueryIntent::Recall => {
                out.push_str("Focus on accurately recalling the requested information. ");
                out.push_str("Cite specific sources and times when available. ");
            }
            super::analyzer::QueryIntent::TemporalDiff => {
                out.push_str("Compare the states across the referenced time periods. ");
                out.push_str("Highlight what changed and when. ");
            }
            super::analyzer::QueryIntent::SystemQuery => {
                out.push_str("Provide accurate system state information. ");
                out.push_str("Include relevant timestamps and snapshots. ");
            }
            _ => {
                out.push_str("Be helpful and concise. ");
            }
        }

        out.push_str("If information comes from a specific time, mention when. ");
        out.push_str("If you're uncertain about something, say so.");
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

    fn create_analysis(intent: QueryIntent) -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test query".to_string(),
            intent,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        }
    }

    fn create_source(content: &str) -> ContextSource {
        ContextSource {
            source_type: ContextSourceType::Knowledge,
            content: content.to_string(),
            relevance: 1.0,
            entity_id: None,
        }
    }

    #[test]
    fn test_augment_basic_structure() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let analysis = create_analysis(QueryIntent::Question);
        let source = create_source("test content");

        let result = augmenter.augment("test query", &[source], &analysis)?;

        assert!(result.contains("# Tardis AI Assistant"));
        assert!(result.contains("## Retrieved Context"));
        assert!(result.contains("### Knowledge 1 (relevance: 1.00)"));
        assert!(result.contains("test content"));
        assert!(result.contains("## User Query"));
        assert!(result.contains("test query"));
        assert!(result.contains("## Instructions"));
        Ok(())
    }

    #[test]
    fn test_augment_empty_context() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let analysis = create_analysis(QueryIntent::Question);

        let result = augmenter.augment("test query", &[], &analysis)?;

        assert!(!result.contains("## Retrieved Context"));
        assert!(result.contains("## User Query"));
        Ok(())
    }

    #[test]
    fn test_augment_truncation() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let analysis = create_analysis(QueryIntent::Question);

        // 4096 tokens * 4 chars/token = 16384 chars.
        // Let's create a source that is larger than that.
        let big_content = "a".repeat(20_000);
        let source1 = create_source(&big_content);
        let source2 = create_source("should be truncated");

        let result = augmenter.augment("test query", &[source1, source2], &analysis)?;

        assert!(result.contains("more sources truncated"));

        assert!(result.contains("2 more sources truncated"));
        assert!(!result.contains(&big_content));
        Ok(())
    }

    #[test]
    fn test_augment_truncation_partial() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let analysis = create_analysis(QueryIntent::Question);

        // 2000 tokens (8000 chars) -> Fits
        let content1 = "a".repeat(8000);
        let source1 = create_source(&content1);

        // 3000 tokens (12000 chars) -> 2000 + 3000 = 5000 > 4096 -> Fits? No.
        let content2 = "b".repeat(12000);
        let source2 = create_source(&content2);

        let result = augmenter.augment("test query", &[source1, source2], &analysis)?;

        // Source 1 should be present
        assert!(result.contains("### Knowledge 1"));

        // Source 2 should be truncated
        assert!(result.contains("1 more sources truncated"));
        assert!(!result.contains(&content2));
        Ok(())
    }

    #[test]
    fn test_instructions_vary_by_intent() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let intents = vec![
            (QueryIntent::Recall, "accurately recalling"),
            (QueryIntent::TemporalDiff, "Compare the states"),
            (QueryIntent::SystemQuery, "system state information"),
            (QueryIntent::Question, "Be helpful and concise"),
        ];

        for (intent, expected_phrase) in intents {
            let analysis = create_analysis(intent);
            let result = augmenter.augment("test query", &[], &analysis)?;

            assert!(
                result.contains(expected_phrase),
                "Intent {:?} did not produce phrase '{}'",
                analysis.intent,
                expected_phrase
            );
        }
        Ok(())
    }

    #[test]
    fn test_system_context_includes_time() -> Result<(), Box<dyn std::error::Error>> {
        let augmenter = ContextAugmenter::new();
        let mut analysis = create_analysis(QueryIntent::Question);
        analysis.temporal_description = Some("yesterday".to_string());

        let result = augmenter.augment("test query", &[], &analysis)?;

        assert!(result.contains("Current time:"));
        assert!(result.contains("Query temporal context: yesterday"));
        Ok(())
    }
}
