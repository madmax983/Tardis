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

impl IntentRule {
    fn matches(&self, query_lower: &str) -> bool {
        // If required keywords are missing, fail fast.
        if !self.required.iter().all(|k| query_lower.contains(k)) {
            return false;
        }

        // If there are optional keywords, one must match.
        self.any.is_empty() || self.any.iter().any(|k| query_lower.contains(k))
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
    /// use chrono::Utc;
    ///
    /// let analyzer = QueryAnalyzer::new();
    /// let query = "What did we do yesterday?";
    /// let analysis = analyzer.analyze(query).unwrap();
    ///
    /// assert_eq!(analysis.intent, QueryIntent::Recall);
    /// assert!(!analysis.temporal_refs.is_empty());
    /// assert_eq!(analysis.temporal_refs[0].text, "yesterday");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if analysis fails.
    pub fn analyze(&self, query: &str) -> ChronosResult<AnalyzedQuery> {
        let lower = query.to_lowercase();
        let intent = self.classify_intent(&lower);
        let temporal_refs = self.extract_temporal_refs(&lower, Utc::now());
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
        INTENT_RULES
            .iter()
            .find(|rule| rule.matches(query_lower))
            .map_or_else(
                || {
                    if query_lower.ends_with('?') {
                        QueryIntent::Question
                    } else {
                        QueryIntent::Chat
                    }
                },
                |rule| rule.intent.clone(),
            )
    }

    /// Extract temporal references from a query.
    #[allow(clippy::unused_self)]
    fn extract_temporal_refs(&self, query_lower: &str, now: DateTime<Utc>) -> Vec<TemporalRef> {
        // TODO: Add more sophisticated temporal extraction
        // - NLP-based extraction
        // - Absolute date parsing
        // - Event-based references

        TEMPORAL_RULES
            .iter()
            .filter(|rule| query_lower.contains(rule.keyword))
            .map(|rule| TemporalRef {
                text: rule.keyword.to_string(),
                resolved: rule.resolve(now),
                ref_type: rule.ref_type.clone(),
            })
            .collect()
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
        use std::fmt::Write;

        if refs.is_empty() {
            return "current time".to_string();
        }

        let mut s = String::new();
        for (i, r) in refs.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            // Ignore write errors on String as it shouldn't fail unless OOM
            let _ = write!(s, "{} ({})", r.text, r.resolved.format("%Y-%m-%d"));
        }
        s
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
            analyzer.classify_intent("remember that i like pizza"),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent("please remember this conversation"),
            QueryIntent::Remember
        );
        assert_eq!(
            analyzer.classify_intent("what did we discuss yesterday?"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("recall the meeting notes"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("what was the result?"),
            QueryIntent::Recall
        );
        assert_eq!(
            analyzer.classify_intent("how has the project changed?"),
            QueryIntent::TemporalDiff
        );
        assert_eq!(
            analyzer.classify_intent("show me the system state"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent("take a snapshot"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            analyzer.classify_intent("is this a question?"),
            QueryIntent::Question
        );
        assert_eq!(analyzer.classify_intent("just chatting"), QueryIntent::Chat);
    }

    #[test]
    fn test_extract_temporal_refs() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();

        let refs = analyzer.extract_temporal_refs("what happened yesterday?", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "yesterday");
        assert_eq!(refs[0].ref_type, TemporalRefType::Relative);

        let refs = analyzer.extract_temporal_refs("check last week logs", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "last week");

        let refs = analyzer.extract_temporal_refs("do it today", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].text, "today");

        let refs = analyzer.extract_temporal_refs("yesterday and today", now);
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
        assert_eq!(refs[0].resolved.to_rfc3339(), "2024-03-14T12:00:00+00:00");

        // "last week" should be 2024-03-08 12:00:00 UTC
        let refs = analyzer.extract_temporal_refs("last week", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].resolved.to_rfc3339(), "2024-03-08T12:00:00+00:00");
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
