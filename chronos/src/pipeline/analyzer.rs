//! Query analysis for Chronos.
//!
//! This module handles the interpretation of natural language queries into structured [`AnalyzedQuery`] objects.
//! It uses a rule-based approach to:
//! 1. **Classify Intent**: Determines what the user wants (e.g., [`QueryIntent::Recall`], [`QueryIntent::Remember`]).
//!    - Uses keyword matching defined in internal rules (e.g., "remember" -> `Remember`).
//! 2. **Extract Temporal References**: Finds time-related terms (e.g., "yesterday", "last week").
//!    - Resolves relative times to absolute UTC timestamps.

use crate::error::ChronosResult;
use chrono::{DateTime, Duration, Utc};
use tardis_common::temporal::TemporalReference;

/// Analyzed query with extracted metadata.
#[derive(Debug, Clone)]
pub struct AnalyzedQuery {
    /// Original query text.
    pub text: String,
    /// Detected intent.
    pub intent: QueryIntent,
    /// Extracted temporal references.
    pub temporal_refs: Vec<TemporalReference>,
    /// Human-readable temporal context.
    pub temporal_description: Option<String>,
    /// Extracted entity mentions.
    pub entities: Vec<String>,
}

/// Query intent classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// General question.
    ///
    /// Example: "Why is the sky blue?"
    Question,
    /// Recall past information.
    ///
    /// Example: "What did we talk about yesterday?"
    Recall,
    /// Store new information.
    ///
    /// Example: "Remember that my favorite color is blue."
    Remember,
    /// Compare across time.
    ///
    /// Example: "How has the codebase changed since last week?"
    TemporalDiff,
    /// System state query.
    ///
    /// Example: "What is the CPU usage?"
    SystemQuery,
    /// General conversation.
    ///
    /// Example: "Hello there!"
    Chat,
}

enum TimeOffset {
    Days(i64),
    Weeks(i64),
    None,
}

struct TemporalRule {
    keyword: &'static str,
    offset: TimeOffset,
}

impl TemporalRule {
    fn resolve(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self.offset {
            TimeOffset::Days(d) => now - Duration::days(d),
            TimeOffset::Weeks(w) => now - Duration::weeks(w),
            TimeOffset::None => now,
        }
    }
}

const TEMPORAL_RULES: &[TemporalRule] = &[
    TemporalRule {
        keyword: "yesterday",
        offset: TimeOffset::Days(1),
    },
    TemporalRule {
        keyword: "last week",
        offset: TimeOffset::Weeks(1),
    },
    TemporalRule {
        keyword: "today",
        offset: TimeOffset::None,
    },
];

struct IntentRule {
    required: &'static [&'static str],
    any: &'static [&'static str],
    intent: QueryIntent,
}

impl IntentRule {
    fn matches(&self, query_lower: &str) -> bool {
        let has_required = self.required.iter().all(|k| query_lower.contains(k));
        let has_any = self.any.is_empty() || self.any.iter().any(|k| query_lower.contains(k));
        has_required && has_any
    }
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
    /// # Examples
    ///
    /// ```
    /// use tardis_chronos::pipeline::{QueryAnalyzer, QueryIntent};
    /// use tardis_common::temporal::TemporalReference;
    /// use chrono::Utc;
    ///
    /// let analyzer = QueryAnalyzer::new();
    /// let query = "What did we do yesterday?";
    /// let analysis = analyzer.analyze(query).unwrap();
    ///
    /// assert_eq!(analysis.intent, QueryIntent::Recall);
    /// assert!(!analysis.temporal_refs.is_empty());
    /// match &analysis.temporal_refs[0] {
    ///    TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
    ///    _ => panic!("Expected relative reference"),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails.
    pub fn analyze(&self, query: &str) -> ChronosResult<AnalyzedQuery> {
        // Optimization: Hoist to_lowercase() to avoid repeating it in helper methods
        let query_lower = query.to_lowercase();
        let intent = self.classify_intent(&query_lower);
        let temporal_refs = self.extract_temporal_refs(&query_lower, Utc::now());
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
    fn classify_intent(&self, query_lower: &str) -> QueryIntent {
        for rule in INTENT_RULES {
            if rule.matches(query_lower) {
                return rule.intent.clone();
            }
        }

        if query_lower.ends_with('?') {
            return QueryIntent::Question;
        }

        QueryIntent::Chat
    }

    /// Extract temporal references from a query.
    ///
    /// Currently uses a rule-based approach to identify keywords like "yesterday", "today",
    /// and "last week".
    ///
    /// # Future Vision
    ///
    /// Planned improvements include:
    /// - NLP-based extraction for complex phrases ("next Tuesday at 5pm").
    /// - Absolute date parsing ("2023-12-25").
    /// - Event-based references ("since the last update").
    #[allow(clippy::unused_self)]
    fn extract_temporal_refs(
        &self,
        query_lower: &str,
        now: DateTime<Utc>,
    ) -> Vec<TemporalReference> {
        let mut refs = Vec::new();

        for rule in TEMPORAL_RULES {
            if query_lower.contains(rule.keyword) {
                let resolved = rule.resolve(now);

                refs.push(TemporalReference::Relative {
                    text: rule.keyword.to_string(),
                    resolved,
                });
            }
        }

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
    fn describe_temporal_context(&self, refs: &[TemporalReference]) -> String {
        if refs.is_empty() {
            return "current time".to_string();
        }

        refs.iter()
            .map(|r| match r {
                TemporalReference::Relative { text, resolved } => {
                    format!("{} ({})", text, resolved.format("%Y-%m-%d"))
                }
                TemporalReference::Absolute(resolved) => resolved.format("%Y-%m-%d").to_string(),
                TemporalReference::EventBased { event, resolved } => {
                    if let Some(res) = resolved {
                        format!("{} ({})", event, res.format("%Y-%m-%d"))
                    } else {
                        event.clone()
                    }
                }
                TemporalReference::Implicit => "implicit".to_string(),
            })
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
            analyzer.classify_intent(&"Remember that I like pizza".to_lowercase()),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent(&"Please remember this conversation".to_lowercase()),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent(&"What did we discuss yesterday?".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent(&"Recall the meeting notes".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent(&"What was the result?".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent(&"How has the project changed?".to_lowercase()),
            QueryIntent::TemporalDiff
        );
        assert_eq!(
            analyzer.classify_intent(&"Show me the system state".to_lowercase()),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent(&"Take a snapshot".to_lowercase()),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent(&"Is this a question?".to_lowercase()),
            QueryIntent::Question
        );
        assert_eq!(
            analyzer.classify_intent(&"Just chatting".to_lowercase()),
            QueryIntent::Chat
        );
    }

    #[test]
    fn test_extract_temporal_refs() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();

        let refs = analyzer.extract_temporal_refs(&"What happened yesterday?".to_lowercase(), now);
        assert_eq!(refs.len(), 1);

        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
            _ => panic!("Expected relative"),
        }

        let refs = analyzer.extract_temporal_refs(&"Check last week logs".to_lowercase(), now);
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "last week"),
            _ => panic!("Expected relative"),
        }

        let refs = analyzer.extract_temporal_refs(&"Do it today".to_lowercase(), now);
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "today"),
            _ => panic!("Expected relative"),
        }

        let refs = analyzer.extract_temporal_refs(&"Yesterday and today".to_lowercase(), now);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_extract_temporal_refs_resolved() {
        let analyzer = QueryAnalyzer::new();
        // Use a fixed date for deterministic testing
        // 2024-03-15 12:00:00 UTC
        let now = DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // "yesterday" should be 2024-03-14 12:00:00 UTC
        let refs = analyzer.extract_temporal_refs("yesterday", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-14T12:00:00+00:00"
        );

        // "last week" should be 2024-03-08 12:00:00 UTC
        let refs = analyzer.extract_temporal_refs("last week", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-08T12:00:00+00:00"
        );
    }

    #[test]
    fn test_describe_temporal_context() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();
        let refs = analyzer.extract_temporal_refs("yesterday", now);
        let desc = analyzer.describe_temporal_context(&refs);
        assert!(desc.contains("yesterday"));
    }
}
