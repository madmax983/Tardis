//! Query analysis for Chronos.

use crate::error::ChronosResult;
use chrono::{DateTime, Duration, Utc};

/// Analyzed query with extracted metadata.
#[derive(Debug, Clone)]
pub struct AnalyzedQuery {
    /// Original query text.
    pub text: String,
    /// Detected intent.
    pub intent: QueryIntent,
    /// Extracted temporal references.
    pub temporal_refs: Vec<TemporalRef>,
    /// Human-readable temporal context.
    pub temporal_description: Option<String>,
    /// Extracted entity mentions.
    pub entities: Vec<String>,
}

/// Query intent classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// General question.
    Question,
    /// Recall past information.
    Recall,
    /// Store new information.
    Remember,
    /// Compare across time.
    TemporalDiff,
    /// System state query.
    SystemQuery,
    /// General conversation.
    Chat,
}

/// A temporal reference in the query.
#[derive(Debug, Clone)]
pub struct TemporalRef {
    /// Original text.
    pub text: String,
    /// Resolved timestamp.
    pub resolved: DateTime<Utc>,
    /// Type of reference.
    pub ref_type: TemporalRefType,
}

/// Type of temporal reference.
#[derive(Debug, Clone)]
pub enum TemporalRefType {
    /// Relative (e.g., "yesterday").
    Relative,
    /// Absolute (e.g., "March 15").
    Absolute,
    /// Event-based (e.g., "before the update").
    EventBased,
}

/// Query analyzer.
#[derive(Debug)]
pub struct QueryAnalyzer {
    // Configuration
}

impl QueryAnalyzer {
    /// Create a new query analyzer.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Analyze a query.
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails.
    pub fn analyze(&self, query: &str) -> ChronosResult<AnalyzedQuery> {
        let intent = Self::classify_intent(query);
        let temporal_refs = Self::extract_temporal_refs(query);
        let entities = Self::extract_entities(query);

        let temporal_description = if temporal_refs.is_empty() {
            None
        } else {
            Some(Self::describe_temporal_context(&temporal_refs))
        };

        Ok(AnalyzedQuery {
            text: query.to_string(),
            intent,
            temporal_refs,
            temporal_description,
            entities,
        })
    }

    /// Classify the intent of a query.
    fn classify_intent(query: &str) -> QueryIntent {
        let lower = query.to_lowercase();

        if lower.contains("remember that") || lower.contains("remember this") {
            return QueryIntent::Remember;
        }

        if lower.contains("what did we") || lower.contains("recall") || lower.contains("what was") {
            return QueryIntent::Recall;
        }

        if lower.contains("how has") && lower.contains("changed") {
            return QueryIntent::TemporalDiff;
        }

        if lower.contains("system state") || lower.contains("snapshot") {
            return QueryIntent::SystemQuery;
        }

        if lower.ends_with('?') {
            return QueryIntent::Question;
        }

        QueryIntent::Chat
    }

    /// Extract temporal references from a query.
    fn extract_temporal_refs(query: &str) -> Vec<TemporalRef> {
        let mut refs = Vec::new();
        let lower = query.to_lowercase();
        let now = Utc::now();

        // Simple pattern matching for common temporal references
        if lower.contains("yesterday") {
            refs.push(TemporalRef {
                text: "yesterday".to_string(),
                resolved: now - Duration::days(1),
                ref_type: TemporalRefType::Relative,
            });
        }

        if lower.contains("last week") {
            refs.push(TemporalRef {
                text: "last week".to_string(),
                resolved: now - Duration::weeks(1),
                ref_type: TemporalRefType::Relative,
            });
        }

        if lower.contains("today") {
            refs.push(TemporalRef {
                text: "today".to_string(),
                resolved: now,
                ref_type: TemporalRefType::Relative,
            });
        }

        // TODO: Add more sophisticated temporal extraction
        // - NLP-based extraction
        // - Absolute date parsing
        // - Event-based references

        refs
    }

    /// Extract entity mentions from a query.
    const fn extract_entities(_query: &str) -> Vec<String> {
        // TODO: Implement NER or pattern matching
        // For now, just return empty
        Vec::new()
    }

    /// Generate human-readable description of temporal context.
    fn describe_temporal_context(refs: &[TemporalRef]) -> String {
        if refs.is_empty() {
            return "current time".to_string();
        }

        refs.iter()
            .map(|r| format!("{} ({})", r.text, r.resolved.format("%Y-%m-%d")))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Default for QueryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
