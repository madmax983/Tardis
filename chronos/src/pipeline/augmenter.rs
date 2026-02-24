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
//! # Token Budgeting & Silent Truncation
//!
//! The augmenter operates with a strict token limit (defined in `RagConfig`) to prevent
//! overflowing the LLM's context window. This is enforced via a **Silent Truncation** policy:
//!
//! 1.  **Strict Limit**: The budget is hard-capped (no "soft" overflow).
//! 2.  **Heuristic**: Tokens are estimated at **4 characters per token**.
//! 3.  **Priority**: Sources are processed in order of relevance (assumed pre-sorted).
//! 4.  **Truncation**:
//!     -   If a source fits entirely, it is included.
//!     -   If it partially fits, it is **silently truncated** to fill the remaining budget.
//!     -   Any subsequent sources are **dropped entirely**, and a summary note
//!         (e.g., "... (N more sources truncated)") is appended.

use super::{ContextSource, RagConfig};
use crate::error::ChronosResult;
use crate::pipeline::analyzer::AnalyzedQuery;
use chrono::Utc;
use std::fmt::Write;

/// Augment a prompt with retrieved context.
///
/// # Silent Truncation
///
/// This function enforces the token budget defined in `config.max_context_tokens`.
/// If the context exceeds this limit:
/// 1. Sources are processed in order.
/// 2. If a source partially fits, it is **silently truncated** to fill the remaining space.
/// 3. Subsequent sources are **dropped entirely**.
/// 4. A summary note (e.g., "... (N more sources truncated)") is appended.
///
/// # Examples
///
/// ```
/// use tardis_chronos::pipeline::{augment, ContextSource, ContextSourceType, RagConfig};
/// use tardis_chronos::pipeline::{AnalyzedQuery, QueryIntent};
/// use tardis_common::temporal::TemporalReference;
/// use chrono::Utc;
///
/// // 1. Setup the input data
/// let config = RagConfig::default();
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
/// let result = augment(prompt, &context, &analysis, &config).unwrap();
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
#[allow(clippy::items_after_statements)]
#[allow(clippy::uninlined_format_args)]
pub fn augment(
    prompt: &str,
    context: &[ContextSource],
    analysis: &AnalyzedQuery,
    config: &RagConfig,
) -> ChronosResult<String> {
    let max_context_tokens = config.max_context_tokens;

    // Hard limit to prevent DoS via excessive allocation
    const MAX_TOKENS_LIMIT: usize = 100_000;
    if max_context_tokens > MAX_TOKENS_LIMIT {
        return Err(crate::error::ChronosError::ContextAssemblyFailed(format!(
            "max_context_tokens {} exceeds limit {}",
            max_context_tokens, MAX_TOKENS_LIMIT
        )));
    }

    // Bolt: Pre-allocate buffer to avoid re-allocations.
    // 4 chars per token + 1KB overhead for system prompts/instructions.
    // Use checked arithmetic to prevent overflow panic
    let capacity = max_context_tokens
        .checked_mul(4)
        .and_then(|c| c.checked_add(1024))
        .ok_or_else(|| {
            crate::error::ChronosError::ContextAssemblyFailed("Capacity overflow".to_string())
        })?;

    let mut formatter = ContextFormatter::new(capacity);

    // System context
    #[cfg(feature = "nova")]
    let persona = config.persona.as_deref();
    #[cfg(not(feature = "nova"))]
    let persona = None;

    formatter.add_system_context(analysis, persona);

    // Retrieved context
    if !context.is_empty() {
        formatter.add_retrieved_context(context, max_context_tokens);
    }

    // User query
    formatter.add_user_query(prompt);

    // Instructions
    formatter.add_instructions(analysis);

    Ok(formatter.finish())
}

/// A helper struct to format the context buffer.
struct ContextFormatter {
    buffer: String,
}

impl ContextFormatter {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    fn finish(self) -> String {
        self.buffer
    }

    /// Write system context header to buffer.
    fn add_system_context(&mut self, analysis: &AnalyzedQuery, persona: Option<&str>) {
        if let Some(p) = persona {
            let _ = writeln!(self.buffer, "# {p}\n");
        } else {
            self.buffer.push_str("# Tardis AI Assistant\n\n");
        }
        // Bolt: Optimized format string usage
        let _ = writeln!(
            self.buffer,
            "Current time: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(ref temporal) = analysis.temporal_description {
            let _ = writeln!(self.buffer, "Query temporal context: {temporal}");
        }
    }

    /// Write retrieved context to buffer with truncation logic.
    fn add_retrieved_context(&mut self, context: &[ContextSource], max_tokens: usize) {
        self.buffer.push_str("\n## Retrieved Context\n\n");

        let mut token_estimate = 0;

        for (i, source) in context.iter().enumerate() {
            let source_tokens = estimate_tokens(&source.content);

            if token_estimate + source_tokens > max_tokens {
                // Calculate remaining budget
                let remaining_tokens = max_tokens.saturating_sub(token_estimate);
                let chars_to_take = remaining_tokens * 4;

                // If we have space for at least some content, include it partially
                if chars_to_take > 0 {
                    let truncated_content = truncate_string(&source.content, chars_to_take);
                    let _ = writeln!(
                        self.buffer,
                        "### {} {} (relevance: {:.2})\n{}...\n",
                        source.source_type,
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
                        self.buffer,
                        "\n... ({sources_fully_dropped} more sources truncated)"
                    );
                }
                break;
            }

            let _ = writeln!(
                self.buffer,
                "### {} {} (relevance: {:.2})\n{}\n",
                source.source_type,
                i + 1,
                source.relevance,
                source.content
            );

            token_estimate += source_tokens;
        }
    }

    fn add_user_query(&mut self, prompt: &str) {
        self.buffer.push_str("\n## User Query\n\n");
        self.buffer.push_str(prompt);
    }

    /// Write response instructions to buffer based on query analysis.
    fn add_instructions(&mut self, analysis: &AnalyzedQuery) {
        self.buffer.push_str("\n\n## Instructions\n\n");
        self.buffer
            .push_str("Respond based on the context provided. ");

        match analysis.intent {
            super::analyzer::QueryIntent::Recall => {
                self.buffer
                    .push_str("Focus on accurately recalling the requested information. ");
                self.buffer
                    .push_str("Cite specific sources and times when available. ");
            }
            super::analyzer::QueryIntent::TemporalDiff => {
                self.buffer
                    .push_str("Compare the states across the referenced time periods. ");
                self.buffer.push_str("Highlight what changed and when. ");
            }
            super::analyzer::QueryIntent::SystemQuery => {
                self.buffer
                    .push_str("Provide accurate system state information. ");
                self.buffer
                    .push_str("Include relevant timestamps and snapshots. ");
            }
            _ => {
                self.buffer.push_str("Be helpful and concise. ");
            }
        }

        self.buffer
            .push_str("If information comes from a specific time, mention when. ");
        self.buffer
            .push_str("If you're uncertain about something, say so.");
    }
}

/// Estimate tokens from text (4 chars per token).
const fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Truncate string to a maximum number of characters, respecting UTF-8 boundaries.
fn truncate_string(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
        let config = RagConfig {
            max_context_tokens: 10, // 40 chars max
            ..RagConfig::default()
        };

        // Create a source with 50 chars (should be truncated)
        // 1 token = 4 chars, so 10 tokens = 40 chars.
        // Source has 50 chars.
        let content = "a".repeat(50);
        let source = create_mock_source(&content);
        let analysis = create_mock_analysis();

        let result = augment("query", &[source], &analysis, &config).unwrap();

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
        let config = RagConfig {
            max_context_tokens: 15, // 60 chars max
            ..RagConfig::default()
        };

        // Source 1: 20 chars (5 tokens)
        let s1 = create_mock_source(&"a".repeat(20));
        // Source 2: 50 chars (12.5 tokens).
        // Remaining budget: 15 - 5 = 10 tokens (40 chars).
        // S2 will be truncated to 40 chars.
        let s2 = create_mock_source(&"b".repeat(50));

        let analysis = create_mock_analysis();

        let result = augment("query", &[s1, s2], &analysis, &config).unwrap();

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
        let config = RagConfig {
            max_context_tokens: 10, // 40 chars
            ..RagConfig::default()
        };

        // S1: 40 chars (10 tokens). Fits exactly.
        let s1 = create_mock_source(&"a".repeat(40));
        // S2: 10 chars. No budget left. Should be dropped.
        let s2 = create_mock_source(&"b".repeat(10));

        let analysis = create_mock_analysis();

        let result = augment("query", &[s1, s2], &analysis, &config).unwrap();

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
        let config = RagConfig {
            max_context_tokens: 10,
            ..RagConfig::default()
        };
        let analysis = create_mock_analysis();

        let result = augment("query", &[], &analysis, &config).unwrap();

        assert!(
            !result.contains("## Retrieved Context"),
            "Should not have context header"
        );
        assert!(result.contains("## User Query"), "Should have user query");
    }

    #[test]
    fn test_augment_exact_limit() {
        let config = RagConfig {
            max_context_tokens: 10, // 40 chars
            ..RagConfig::default()
        };

        // 40 chars. Fits exactly.
        let content = "a".repeat(40);
        let source = create_mock_source(&content);
        let analysis = create_mock_analysis();

        let result = augment("query", &[source], &analysis, &config).unwrap();

        assert!(result.contains(&content), "Should contain full content");
        assert!(
            !result.contains("truncated"),
            "Should not report truncation"
        );
    }

    #[test]
    fn test_augment_many_small_sources() {
        let config = RagConfig {
            max_context_tokens: 5,
            ..RagConfig::default()
        };

        // Create 20 sources of 3 chars each ("s00", "s01", etc.)
        // Current logic: 3/4 = 0 tokens. All 20 fit.
        // Correct logic: (3+3)/4 = 1 token. Only 5 fit.
        let sources: Vec<ContextSource> = (0..20)
            .map(|i| ContextSource {
                source_type: ContextSourceType::Knowledge,
                content: format!("s{i:02}"),
                relevance: 1.0,
                entity_id: None,
            })
            .collect();

        let analysis = create_mock_analysis();

        let result = augment("query", &sources, &analysis, &config).unwrap();

        // Check if truncation happened correctly
        // With limit=5, we expect 5 sources to be included (indices 0..5)
        for i in 0..5 {
            assert!(
                result.contains(&format!("s{i:02}")),
                "Should contain source {i}"
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

    #[cfg(feature = "nova")]
    #[test]
    fn test_augment_with_persona() {
        let config = RagConfig {
            persona: Some("You are a Dalek.".to_string()),
            ..RagConfig::default()
        };
        let analysis = create_mock_analysis();

        let result = augment("query", &[], &analysis, &config).unwrap();

        assert!(result.contains("# You are a Dalek."));
        assert!(!result.contains("# Tardis AI Assistant"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::panic)]
mod sentry_tests {
    use super::*;
    use crate::pipeline::analyzer::{AnalyzedQuery, QueryIntent};
    use crate::pipeline::{ContextSource, ContextSourceType};

    fn create_mock_analysis(intent: QueryIntent) -> AnalyzedQuery {
        AnalyzedQuery {
            text: "test".to_string(),
            intent,
            temporal_refs: vec![],
            temporal_description: None,
            entities: vec![],
        }
    }

    fn create_mock_source(content: &str) -> ContextSource {
        ContextSource {
            source_type: ContextSourceType::Knowledge,
            content: content.to_string(),
            relevance: 1.0,
            entity_id: None,
        }
    }

    #[test]
    fn test_write_instructions_intent_coverage() {
        let config = RagConfig::default();
        let intents = vec![
            (QueryIntent::Recall, "Focus on accurately recalling"),
            (QueryIntent::TemporalDiff, "Compare the states"),
            (QueryIntent::SystemQuery, "Provide accurate system state"),
            (QueryIntent::Chat, "Be helpful and concise"),
            (QueryIntent::Question, "Be helpful and concise"),
            (QueryIntent::Remember, "Be helpful and concise"),
        ];

        for (intent, expected_phrase) in intents {
            let analysis = create_mock_analysis(intent.clone());
            let result = augment("query", &[], &analysis, &config).unwrap();

            assert!(
                result.contains(expected_phrase),
                "Instructions for {intent:?} should contain '{expected_phrase}'",
            );
        }
    }

    #[test]
    fn test_augment_truncation_boundary_conditions() {
        let config = RagConfig {
            max_context_tokens: 10, // 40 chars
            ..RagConfig::default()
        };

        // Source 1: 39 chars. 10 tokens (39/4 ceil = 10).
        // Remaining budget: 10 - 10 = 0 tokens.
        let s1 = create_mock_source(&"a".repeat(39));

        // Source 2: 1 char. Should be dropped.
        let s2 = create_mock_source("SHOULD_NOT_APPEAR");

        let analysis = create_mock_analysis(QueryIntent::Question);

        let result = augment("query", &[s1, s2], &analysis, &config).unwrap();

        assert!(result.contains(&"a".repeat(39)), "Should contain s1");
        assert!(
            !result.contains("SHOULD_NOT_APPEAR"),
            "Should not contain s2"
        );
        assert!(
            result.contains("1 more sources truncated"),
            "Should report s2 dropped"
        );
    }
}
