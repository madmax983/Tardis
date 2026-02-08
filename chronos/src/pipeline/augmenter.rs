//! Context augmentation for Chronos.
//!
//! This module acts as the "Prompt Engineer" of the RAG pipeline. It takes the
//! raw user query and the retrieved context (facts, conversation history) and
//! assembles them into a structured prompt that guides the LLM to provide
//! accurate, grounded responses.
//!
//! # The Augmentation Strategy
//!
//! 1.  **System Context**: Sets the persona ("Tardis AI Assistant") and injects
//!     the current system time. This is critical for the LLM to understand "now".
//! 2.  **Retrieved Context**: Formats the retrieved documents into a labeled
//!     section, including source type (Knowledge, Conversation) and relevance scores.
//! 3.  **User Query**: The original question.
//! 4.  **Instructions**: Dynamic instructions based on the query intent (e.g.,
//!     "Focus on recall" vs "Compare states").

use super::{ContextSource, ContextSourceType};
use crate::error::ChronosResult;
use crate::pipeline::analyzer::AnalyzedQuery;
use chrono::Utc;
use std::fmt::Write;

/// Context augmenter for building RAG prompts.
///
/// This struct manages token limits and formatting logic for prompt construction.
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
    /// # Examples
    ///
    /// ```
    /// use tardis_chronos::pipeline::{ContextAugmenter, ContextSource, ContextSourceType};
    /// use tardis_chronos::pipeline::{QueryAnalyzer, AnalyzedQuery, QueryIntent};
    /// use chrono::Utc;
    ///
    /// // 1. Setup the input data
    /// let augmenter = ContextAugmenter::new();
    /// let prompt = "Who is the Doctor?";
    ///
    /// // Mock an analyzed query
    /// let analysis = AnalyzedQuery {
    ///     text: prompt.to_string(),
    ///     intent: QueryIntent::Question,
    ///     temporal_refs: vec![],
    ///     temporal_description: None,
    ///     entities: vec![],
    /// };
    ///
    /// // Mock retrieved context
    /// let context = vec![
    ///     ContextSource {
    ///         source_type: ContextSourceType::Knowledge,
    ///         content: "The Doctor is a Time Lord from Gallifrey.".to_string(),
    ///         relevance: 0.95,
    ///         entity_id: None,
    ///     }
    /// ];
    ///
    /// // 2. Generate the augmented prompt
    /// let result = augmenter.augment(prompt, &context, &analysis).unwrap();
    ///
    /// // 3. Verify the structure
    /// assert!(result.contains("# Tardis AI Assistant"));
    /// assert!(result.contains("## Retrieved Context"));
    /// assert!(result.contains("The Doctor is a Time Lord"));
    /// assert!(result.contains("## User Query"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if augmentation fails (e.g., formatting errors).
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
    #[allow(clippy::unused_self)]
    fn build_system_context(&self, analysis: &AnalyzedQuery) -> String {
        let mut ctx = String::new();

        ctx.push_str("# Tardis AI Assistant\n\n");
        // Bolt: Optimized format string usage
        let _ = writeln!(
            ctx,
            "Current time: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(ref temporal) = analysis.temporal_description {
            let _ = writeln!(ctx, "Query temporal context: {temporal}");
        }

        ctx
    }

    /// Format retrieved context.
    fn format_context(&self, context: &[ContextSource]) -> String {
        let mut formatted = String::new();
        let mut token_estimate = 0;

        for (i, source) in context.iter().enumerate() {
            // Rough token estimate (4 chars per token).
            // Use ceiling division to ensure at least 1 token for non-empty content.
            let source_len = source.content.len();
            let source_tokens = if source_len == 0 {
                0
            } else {
                source_len.div_ceil(4)
            };

            let source_type = match source.source_type {
                ContextSourceType::Knowledge => "Knowledge",
                ContextSourceType::Conversation => "Conversation",
                ContextSourceType::SystemState => "System State",
            };

            if token_estimate + source_tokens > self.max_context_tokens {
                // Determine remaining budget
                let remaining_tokens = self.max_context_tokens.saturating_sub(token_estimate);

                // If we have a reasonable amount of space left (e.g., > 10 tokens), include partial content
                let included_partial = if remaining_tokens > 10 {
                    let allowed_chars = remaining_tokens * 4;
                    // Safe truncation using char iterator
                    let truncated_content: String = source
                        .content
                        .chars()
                        .take(allowed_chars)
                        .collect();

                    let _ = writeln!(
                        formatted,
                        "### {source_type} {} (relevance: {:.2})\n{}... (truncated)\n",
                        i + 1,
                        source.relevance,
                        truncated_content
                    );
                    true
                } else {
                    false
                };

                let remaining_sources = if included_partial {
                    context.len() - (i + 1)
                } else {
                    context.len() - i
                };

                if remaining_sources > 0 {
                    let _ = writeln!(
                        formatted,
                        "\n... ({remaining_sources} more sources truncated)"
                    );
                }
                break;
            }

            let _ = writeln!(
                formatted,
                "### {source_type} {} (relevance: {:.2})\n{}\n",
                i + 1,
                source.relevance,
                source.content
            );

            token_estimate += source_tokens;
        }

        formatted
    }

    /// Build response instructions based on query analysis.
    #[allow(clippy::unused_self)]
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::pipeline::analyzer::QueryIntent;

    fn mock_analysis(intent: QueryIntent) -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test query".to_string(),
            intent,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        }
    }

    #[test]
    fn should_include_partial_context_when_source_is_too_large() {
        let augmenter = ContextAugmenter::new();
        // max_context_tokens = 4096
        // 4096 * 4 = 16384 chars.
        // Let's create a source with 20000 chars.
        let huge_content = "a".repeat(20000);
        let context = vec![ContextSource {
            source_type: ContextSourceType::Knowledge,
            content: huge_content.clone(),
            relevance: 1.0,
            entity_id: None,
        }];

        let analysis = mock_analysis(QueryIntent::Question);
        let result = augmenter.augment("test", &context, &analysis).unwrap();

        // Currently, this will fail because the source is dropped entirely.
        // We expect it to contain at least some of the content.
        assert!(result.contains(&"a".repeat(100)), "Should contain partial content");
    }

    #[test]
    fn should_respect_max_tokens_with_multiple_sources() {
        let augmenter = ContextAugmenter::new();
        // Create 6 sources of ~1000 tokens each.
        // Total 6000 tokens > 4096.
        // 0-3 take ~4012 tokens.
        // 4 takes remaining ~84 tokens (partial).
        // 5 is fully truncated.

        let filler = "a".repeat(4000); // ~1000 tokens
        let context: Vec<ContextSource> = (0..6).map(|i| ContextSource {
            source_type: ContextSourceType::Knowledge,
            content: format!("Source {i}: {filler}"),
            relevance: 1.0,
            entity_id: None,
        }).collect();

        let analysis = mock_analysis(QueryIntent::Question);
        let result = augmenter.augment("test", &context, &analysis).unwrap();

        assert!(result.contains("Source 0"));
        assert!(result.contains("Source 1"));
        assert!(result.contains("Source 2"));
        assert!(result.contains("Source 3"));
        // Source 4 should be partially included because we have ~84 tokens budget left
        assert!(result.contains("Source 4"), "Should contain partial Source 4");
        // Source 5 should be fully truncated
        assert!(!result.contains("Source 5"), "Should not contain Source 5");

        // Check for truncation messages
        // "Source 4 ... (truncated)"
        // "... (1 more sources truncated)" (Source 5)
        assert!(result.contains("(truncated)"));
    }
}
