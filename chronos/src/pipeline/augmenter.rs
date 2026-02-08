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
            // Rough token estimate (4 chars per token)
            let source_tokens = source.content.len() / 4;
            if token_estimate + source_tokens > self.max_context_tokens {
                let _ = writeln!(
                    formatted,
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
