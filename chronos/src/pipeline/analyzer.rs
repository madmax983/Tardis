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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalRefType {
    /// Relative (e.g., "yesterday").
    Relative,
    /// Absolute (e.g., "March 15").
    Absolute,
    /// Event-based (e.g., "before the update").
    EventBased,
}

enum TimeOffset {
    Days(i64),
    Weeks(i64),
    None,
}

struct TemporalRule {
    keyword: &'static str,
    offset: TimeOffset,
    ref_type: TemporalRefType,
}

const TEMPORAL_RULES: &[TemporalRule] = &[
    TemporalRule {
        keyword: "yesterday",
        offset: TimeOffset::Days(1),
        ref_type: TemporalRefType::Relative,
    },
    TemporalRule {
        keyword: "last week",
        offset: TimeOffset::Weeks(1),
        ref_type: TemporalRefType::Relative,
    },
    TemporalRule {
        keyword: "today",
        offset: TimeOffset::None,
        ref_type: TemporalRefType::Relative,
    },
];

struct IntentRule {
    required: &'static [&'static str],
    any: &'static [&'static str],
    intent: QueryIntent,
}

const INTENT_RULES: &[IntentRule] = &[
    IntentRule {
        required: &[],
        any: &["remember that", "remember this"],
        intent: QueryIntent::Remember,
    },
    IntentRule {
        required: &[],
        any: &["what did we", "recall", "what was"],
        intent: QueryIntent::Recall,
    },
    IntentRule {
        required: &["how has", "changed"],
        any: &[],
        intent: QueryIntent::TemporalDiff,
    },
    IntentRule {
        required: &[],
        any: &["system state", "snapshot"],
        intent: QueryIntent::SystemQuery,
    },
];

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
        let intent = self.classify_intent(query);
        let temporal_refs = self.extract_temporal_refs(query);
        let entities = self.extract_entities(query);

        let temporal_description = if temporal_refs.is_empty() {
            None
        } else {
            Some(self.describe_temporal_context(&temporal_refs))
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
    #[allow(clippy::unused_self)]
    fn classify_intent(&self, query: &str) -> QueryIntent {
        let lower = query.to_lowercase();

        for rule in INTENT_RULES {
            let has_required = rule.required.iter().all(|k| lower.contains(k));
            let has_any = rule.any.is_empty() || rule.any.iter().any(|k| lower.contains(k));

            if has_required && has_any {
                return rule.intent.clone();
            }
        }

        if lower.ends_with('?') {
            return QueryIntent::Question;
        }

        QueryIntent::Chat
    }

    /// Extract temporal references from a query.
    #[allow(clippy::unused_self)]
    fn extract_temporal_refs(&self, query: &str) -> Vec<TemporalRef> {
        let mut refs = Vec::new();
        let lower = query.to_lowercase();
        let now = Utc::now();

        for rule in TEMPORAL_RULES {
            if lower.contains(rule.keyword) {
                let resolved = match rule.offset {
                    TimeOffset::Days(d) => now - Duration::days(d),
                    TimeOffset::Weeks(w) => now - Duration::weeks(w),
                    TimeOffset::None => now,
                };

                refs.push(TemporalRef {
                    text: rule.keyword.to_string(),
                    resolved,
                    ref_type: rule.ref_type.clone(),
                });
            }
        }

        // TODO: Add more sophisticated temporal extraction
        // - NLP-based extraction
        // - Absolute date parsing
        // - Event-based references

        refs
    }

    /// Extract entity mentions from a query.
    #[allow(clippy::unused_self)]
    const fn extract_entities(&self, query: &str) -> Vec<String> {
        // TODO: Implement NER or pattern matching
        // For now, just return empty
        let _ = query;
        Vec::new()
    }

    /// Generate human-readable description of temporal context.
    #[allow(clippy::unused_self)]
    fn describe_temporal_context(&self, refs: &[TemporalRef]) -> String {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_intent() {
        let analyzer = QueryAnalyzer::new();

        assert_eq!(
            analyzer.classify_intent("Remember that I like pizza"),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent("Please remember this conversation"),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent("What did we discuss yesterday?"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("Recall the meeting notes"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("What was the result?"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("How has the project changed?"),
            QueryIntent::TemporalDiff
        );
        assert_eq!(
            analyzer.classify_intent("Show me the system state"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent("Take a snapshot"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent("Is this a question?"),
            QueryIntent::Question
        );
        assert_eq!(analyzer.classify_intent("Just chatting"), QueryIntent::Chat);
    }

    #[test]
    fn test_extract_temporal_refs() {
        let analyzer = QueryAnalyzer::new();

        let refs = analyzer.extract_temporal_refs("What happened yesterday?");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "yesterday");
        assert_eq!(refs[0].ref_type, TemporalRefType::Relative);

        let refs = analyzer.extract_temporal_refs("Check last week logs");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "last week");

        let refs = analyzer.extract_temporal_refs("Do it today");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "today");

        let refs = analyzer.extract_temporal_refs("Yesterday and today");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_describe_temporal_context() {
        let analyzer = QueryAnalyzer::new();
        let refs = analyzer.extract_temporal_refs("yesterday");
        let desc = analyzer.describe_temporal_context(&refs);
        assert!(desc.contains("yesterday"));
    }
}
