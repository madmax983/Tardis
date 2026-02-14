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
//!
//! # Token Budgeting
//!
//! The `ContextAugmenter` operates with a strict token limit (default: 4096 tokens) to prevent
//! overflowing the LLM's context window.
//!
//! - It uses a heuristic of **4 characters per token**.
//! - It iterates through retrieved sources in order of relevance (assumed pre-sorted).
//! - If a source fits entirely within the remaining budget, it is included.
//! - If a source partially fits, it is **truncated** to fit the remaining budget.
//! - Any subsequent sources are dropped, and a note "... (N more sources truncated)" is appended.
//!
//! # ⚠️ Silent Truncation Warning
//!
//! Because sources are processed in order of relevance, it is possible for a single large,
//! highly relevant source to consume the entire token budget, causing all subsequent sources
//! to be silently dropped.
//!
//! This is by design: we prioritize high-relevance context over quantity. However, if you are
//! getting "missing context" errors, check if your top results are excessively long.

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
    /// use tardis_common::temporal::TemporalReference;
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
        // Bolt: Pre-allocate buffer to avoid re-allocations.
        // 4 chars per token + 1KB overhead for system prompts/instructions.
        let capacity = self.max_context_tokens * 4 + 1024;
        let mut augmented = String::with_capacity(capacity);

        // System context
        self.write_system_context(&mut augmented, analysis);

        // Retrieved context
        if !context.is_empty() {
            augmented.push_str("\n## Retrieved Context\n\n");
            self.write_context(&mut augmented, context);
        }

        // User query
        augmented.push_str("\n## User Query\n\n");
        augmented.push_str(prompt);

        // Instructions
        augmented.push_str("\n\n## Instructions\n\n");
        self.write_instructions(&mut augmented, analysis);

        Ok(augmented)
    }

    /// Write system context header to buffer.
    #[allow(clippy::unused_self)]
    fn write_system_context(&self, buffer: &mut String, analysis: &AnalyzedQuery) {
        buffer.push_str("# Tardis AI Assistant\n\n");
        // Bolt: Optimized format string usage
        let _ = writeln!(
            buffer,
            "Current time: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(ref temporal) = analysis.temporal_description {
            let _ = writeln!(buffer, "Query temporal context: {temporal}");
        }
    }

    /// Write retrieved context to buffer.
    fn write_context(&self, buffer: &mut String, context: &[ContextSource]) {
        let mut token_estimate = 0;

        for (i, source) in context.iter().enumerate() {
            let source_type = match source.source_type {
                ContextSourceType::Knowledge => "Knowledge",
                ContextSourceType::Conversation => "Conversation",
                ContextSourceType::SystemState => "System State",
            };

            // Rough token estimate (4 chars per token)
            // Ceiling division to ensure non-empty sources cost at least 1 token
            let source_tokens = source.content.len().div_ceil(4);

            if token_estimate + source_tokens > self.max_context_tokens {
                // Calculate remaining budget
                let remaining_tokens = self.max_context_tokens.saturating_sub(token_estimate);
                let chars_to_take = remaining_tokens * 4;

                // If we have space for at least some content, include it partially
                if chars_to_take > 0 {
                    // Bolt optimization: slice string instead of allocating new one
                    let end_index = source
                        .content
                        .char_indices()
                        .map(|(i, _)| i)
                        .nth(chars_to_take)
                        .unwrap_or(source.content.len());

                    let truncated_content = &source.content[..end_index];
                    let _ = writeln!(
                        buffer,
                        "### {source_type} {} (relevance: {:.2})\n{}...\n",
                        i + 1,
                        source.relevance,
                        truncated_content
                    );
                }

                // Report how many sources were fully dropped
                // If we took some content from the current source, we only count subsequent sources.
                // If we took NO content from the current source, we count it as dropped too.
                let sources_fully_dropped = if chars_to_take > 0 {
                    context.len() - (i + 1)
                } else {
                    context.len() - i
                };

                if sources_fully_dropped > 0 {
                    let _ = writeln!(
                        buffer,
                        "\n... ({sources_fully_dropped} more sources truncated)"
                    );
                }
                break;
            }

            let _ = writeln!(
                buffer,
                "### {source_type} {} (relevance: {:.2})\n{}\n",
                i + 1,
                source.relevance,
                source.content
            );

            token_estimate += source_tokens;
        }
    }

    /// Write response instructions to buffer based on query analysis.
    #[allow(clippy::unused_self)]
    fn write_instructions(&self, buffer: &mut String, analysis: &AnalyzedQuery) {
        buffer.push_str("Respond based on the context provided. ");

        match analysis.intent {
            super::analyzer::QueryIntent::Recall => {
                buffer.push_str("Focus on accurately recalling the requested information. ");
                buffer.push_str("Cite specific sources and times when available. ");
            }
            super::analyzer::QueryIntent::TemporalDiff => {
                buffer.push_str("Compare the states across the referenced time periods. ");
                buffer.push_str("Highlight what changed and when. ");
            }
            super::analyzer::QueryIntent::SystemQuery => {
                buffer.push_str("Provide accurate system state information. ");
                buffer.push_str("Include relevant timestamps and snapshots. ");
            }
            _ => {
                buffer.push_str("Be helpful and concise. ");
            }
        }

        buffer.push_str("If information comes from a specific time, mention when. ");
        buffer.push_str("If you're uncertain about something, say so.");
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
    use crate::pipeline::{ContextSource, ContextSourceType};

    fn create_mock_source(content: &str) -> ContextSource {
        ContextSource {
            source_type: ContextSourceType::Knowledge,
            content: content.to_string(),
            relevance: 1.0,
            entity_id: None,
        }
    }

    fn create_mock_analysis() -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test".to_string(),
            intent: QueryIntent::Question,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        }
    }

    #[test]
    fn test_augment_truncates_large_source() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 10,
        }; // 40 chars max

        // Create a source with 50 chars (should be truncated)
        // 1 token = 4 chars, so 10 tokens = 40 chars.
        // Source has 50 chars.
        let content = "a".repeat(50);
        let source = create_mock_source(&content);
        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &[source], &analysis).unwrap();

        // Should contain the truncated content (40 chars)
        assert!(
            result.contains(&content[..40]),
            "Result should contain truncated content"
        );
        // Should NOT contain the full content
        assert!(
            !result.contains(&content),
            "Result should not contain full content"
        );
        // Should indicate truncation with ellipsis
        assert!(result.contains("..."), "Result should contain ellipsis");
        // Should NOT say "1 more sources truncated" because we partially included it and there are no MORE sources.
        assert!(
            !result.contains("sources truncated"),
            "Should not report dropped sources when none were fully dropped"
        );
    }

    #[test]
    fn test_augment_multiple_sources_partial() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 15,
        }; // 60 chars max

        // Source 1: 20 chars (5 tokens)
        let s1 = create_mock_source(&"a".repeat(20));
        // Source 2: 50 chars (12.5 tokens).
        // Remaining budget: 15 - 5 = 10 tokens (40 chars).
        // S2 will be truncated to 40 chars.
        let s2 = create_mock_source(&"b".repeat(50));

        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &[s1, s2], &analysis).unwrap();

        assert!(
            result.contains(&"a".repeat(20)),
            "Should contain full first source"
        );
        assert!(
            result.contains(&"b".repeat(40)),
            "Should contain truncated second source"
        );
        assert!(
            !result.contains(&"b".repeat(50)),
            "Should not contain full second source"
        );
    }

    #[test]
    fn test_augment_multiple_sources_dropped() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 10,
        }; // 40 chars

        // S1: 40 chars (10 tokens). Fits exactly.
        let s1 = create_mock_source(&"a".repeat(40));
        // S2: 10 chars. No budget left. Should be dropped.
        let s2 = create_mock_source(&"b".repeat(10));

        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &[s1, s2], &analysis).unwrap();

        assert!(
            result.contains(&"a".repeat(40)),
            "Should contain first source"
        );
        assert!(
            !result.contains(&"b".repeat(10)),
            "Should not contain second source"
        );
        assert!(
            result.contains("1 more sources truncated"),
            "Should report dropped source"
        );
    }

    #[test]
    fn test_augment_empty_context() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 10,
        };
        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &[], &analysis).unwrap();

        assert!(
            !result.contains("## Retrieved Context"),
            "Should not have context header"
        );
        assert!(result.contains("## User Query"), "Should have user query");
    }

    #[test]
    fn test_augment_exact_limit() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 10,
        }; // 40 chars

        // 40 chars. Fits exactly.
        let content = "a".repeat(40);
        let source = create_mock_source(&content);
        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &[source], &analysis).unwrap();

        assert!(result.contains(&content), "Should contain full content");
        assert!(
            !result.contains("truncated"),
            "Should not report truncation"
        );
    }

    #[test]
    fn test_augment_many_small_sources() {
        let augmenter = ContextAugmenter {
            max_context_tokens: 5,
        };

        // Create 20 sources of 3 chars each ("s00", "s01", etc.)
        // Current logic: 3/4 = 0 tokens. All 20 fit.
        // Correct logic: (3+3)/4 = 1 token. Only 5 fit.
        let sources: Vec<ContextSource> = (0..20)
            .map(|i| ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: format!("s{:02}", i),
                relevance: 1.0,
                entity_id: None,
            })
            .collect();

        let analysis = create_mock_analysis();

        let result = augmenter.augment("query", &sources, &analysis).unwrap();

        // Check if truncation happened correctly
        // With limit=5, we expect 5 sources to be included (indices 0..5)
        for i in 0..5 {
            assert!(
                result.contains(&format!("s{:02}", i)),
                "Should contain source {}",
                i
            );
        }

        // The 6th source (index 5) should be excluded
        assert!(
            !result.contains("s05"),
            "Should not contain source 5 (limit exceeded)"
        );

        // Should report dropped sources
        assert!(
            result.contains("15 more sources truncated"),
            "Should report 15 dropped sources"
        );
    }
}
