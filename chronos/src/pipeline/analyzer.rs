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

/// Analyze a query.
///
/// # Examples
///
/// **Recall Query with Temporal Reference:**
/// ```
/// use tardis_chronos::pipeline::{analyze, QueryIntent};
/// use tardis_common::temporal::TemporalReference;
///
/// let analysis = analyze("What did we do yesterday?").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::Recall);
///
/// // Verify temporal extraction
/// let reference = &analysis.temporal_refs[0];
/// match reference {
///    TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
///    _ => panic!("Expected relative reference"),
/// }
/// ```
///
/// **Storing a Memory:**
/// ```
/// use tardis_chronos::pipeline::{analyze, QueryIntent};
///
/// let analysis = analyze("Remember that the sky is blue").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::Remember);
/// ```
///
/// **System Introspection:**
/// ```
/// use tardis_chronos::pipeline::{analyze, QueryIntent};
///
/// let analysis = analyze("Take a system snapshot").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::SystemQuery);
/// ```
///
/// # Errors
///
/// Returns an error if analysis fails.
pub fn analyze(query: &str) -> ChronosResult<AnalyzedQuery> {
    analyze_at(query, Utc::now())
}

/// Analyze a query at a specific point in time.
///
/// This is useful for deterministic testing or processing historical queries.
///
/// # Errors
///
/// Returns an error if analysis fails.
pub fn analyze_at(query: &str, now: DateTime<Utc>) -> ChronosResult<AnalyzedQuery> {
    // Optimization: Hoist to_lowercase() to avoid repeating it in helper methods
    let query_lower = query.to_lowercase();
    let intent = classify_intent(&query_lower);
    let temporal_refs = extract_temporal_refs(&query_lower, now);
    let entities = extract_entities(query);

    let temporal_description = if temporal_refs.is_empty() {
        None
    } else {
        Some(describe_temporal_context(&temporal_refs))
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
fn classify_intent(query_lower: &str) -> QueryIntent {
    INTENT_RULES.iter().find(|rule| rule.matches(query_lower)).map_or_else(
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
fn extract_temporal_refs(query_lower: &str, now: DateTime<Utc>) -> Vec<TemporalReference> {
    // TODO: Add more sophisticated temporal extraction
    // - NLP-based extraction
    // - Absolute date parsing
    // - Event-based references

    TEMPORAL_RULES
        .iter()
        .filter(|rule| query_lower.contains(rule.keyword))
        .map(|rule| TemporalReference::Relative {
            text: rule.keyword.to_string(),
            resolved: rule.resolve(now),
        })
        .collect()
}

/// Extract entity mentions from a query.
const fn extract_entities(query: &str) -> Vec<String> {
    // TODO: Implement NER or pattern matching
    // For now, just return empty
    let _ = query;
    Vec::new()
}

/// Generate human-readable description of temporal context.
fn describe_temporal_context(refs: &[TemporalReference]) -> String {
    if refs.is_empty() {
        return "current time".to_string();
    }

    refs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_intent() {
        assert_eq!(
            classify_intent(&"Remember that I like pizza".to_lowercase()),
            QueryIntent::Remember
        );
        assert_eq!(
            classify_intent(&"Please remember this conversation".to_lowercase()),
            QueryIntent::Remember
        );
        assert_eq!(
            classify_intent(&"What did we discuss yesterday?".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            classify_intent(&"Recall the meeting notes".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            classify_intent(&"What was the result?".to_lowercase()),
            QueryIntent::Recall
        );
        assert_eq!(
            classify_intent(&"How has the project changed?".to_lowercase()),
            QueryIntent::TemporalDiff
        );
        assert_eq!(
            classify_intent(&"Show me the system state".to_lowercase()),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            classify_intent(&"Take a snapshot".to_lowercase()),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            classify_intent(&"Is this a question?".to_lowercase()),
            QueryIntent::Question
        );
        assert_eq!(
            classify_intent(&"Just chatting".to_lowercase()),
            QueryIntent::Chat
        );
    }

    #[test]
    fn test_extract_temporal_refs() {
        let now = Utc::now();

        let refs = extract_temporal_refs(&"What happened yesterday?".to_lowercase(), now);
        assert_eq!(refs.len(), 1);

        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
            _ => panic!("Expected relative"),
        }

        let refs = extract_temporal_refs(&"Check last week logs".to_lowercase(), now);
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "last week"),
            _ => panic!("Expected relative"),
        }

        let refs = extract_temporal_refs(&"Do it today".to_lowercase(), now);
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "today"),
            _ => panic!("Expected relative"),
        }

        let refs = extract_temporal_refs(&"Yesterday and today".to_lowercase(), now);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_extract_temporal_refs_resolved() {
        // Use a fixed date for deterministic testing
        // 2024-03-15 12:00:00 UTC
        let now = DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // "yesterday" should be 2024-03-14 12:00:00 UTC
        let refs = extract_temporal_refs("yesterday", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-14T12:00:00+00:00"
        );

        // "last week" should be 2024-03-08 12:00:00 UTC
        let refs = extract_temporal_refs("last week", now);
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-08T12:00:00+00:00"
        );
    }

    #[test]
    fn test_describe_temporal_context() {
        let now = Utc::now();
        let refs = extract_temporal_refs("yesterday", now);
        let desc = describe_temporal_context(&refs);
        assert!(desc.contains("yesterday"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod deterministic_tests {
    use super::*;

    // Helper to get a fixed "now"
    fn get_fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_analyze_at_temporal_resolution() {
        let now = get_fixed_now();

        // "yesterday" -> 2024-03-14
        let res = analyze_at("What happened yesterday?", now).unwrap();
        assert_eq!(res.temporal_refs.len(), 1);
        assert_eq!(
            res.temporal_refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-14T12:00:00+00:00"
        );

        // "last week" -> 2024-03-08
        let res = analyze_at("Check last week logs", now).unwrap();
        assert_eq!(res.temporal_refs.len(), 1);
        assert_eq!(
            res.temporal_refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-08T12:00:00+00:00"
        );

        // "today" -> 2024-03-15
        let res = analyze_at("Do it today", now).unwrap();
        assert_eq!(res.temporal_refs.len(), 1);
        assert_eq!(
            res.temporal_refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-15T12:00:00+00:00"
        );
    }

    #[test]
    fn test_analyze_at_intent_classification() {
        let now = get_fixed_now();

        let cases = vec![
            ("Remember that I like pizza", QueryIntent::Remember),
            ("Please remember this conversation", QueryIntent::Remember),
            ("What did we discuss?", QueryIntent::Recall),
            ("Recall the meeting notes", QueryIntent::Recall),
            ("How has it changed?", QueryIntent::TemporalDiff),
            ("Take a system snapshot", QueryIntent::SystemQuery),
            ("Is this a question?", QueryIntent::Question),
            ("Just chatting", QueryIntent::Chat),
        ];

        for (query, expected) in cases {
            let res = analyze_at(query, now).unwrap();
            assert_eq!(res.intent, expected, "Failed for query: {}", query);
        }
    }

    #[test]
    fn test_analyze_at_mixed_references() {
        let now = get_fixed_now();

        // "yesterday" and "today"
        let res = analyze_at("Compare yesterday and today", now).unwrap();
        assert_eq!(res.temporal_refs.len(), 2);

        // Order depends on implementation (TEMPORAL_RULES order or scan order).
        // TEMPORAL_RULES: yesterday, last week, today.
        // It iterates over rules and checks contains.
        // "yesterday" (rule 1) matches. Added first.
        // "today" (rule 3) matches. Added second.

        let texts: Vec<String> = res
            .temporal_refs
            .iter()
            .map(|r| match r {
                TemporalReference::Relative { text, .. } => text.clone(),
                _ => panic!("Expected relative"),
            })
            .collect();

        assert!(texts.contains(&"yesterday".to_string()));
        assert!(texts.contains(&"today".to_string()));
    }

    #[test]
    fn test_analyze_at_case_insensitivity() {
        let now = get_fixed_now();

        let res = analyze_at("WHAT HAPPENED YESTERDAY?", now).unwrap();
        assert_eq!(res.temporal_refs.len(), 1);
        match &res.temporal_refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
            _ => panic!("Expected relative reference"),
        }
    }

    #[test]
    fn test_analyze_at_no_temporal_reference() {
        let now = get_fixed_now();

        let res = analyze_at("Hello world", now).unwrap();
        assert!(res.temporal_refs.is_empty());
        assert!(res.temporal_description.is_none());
    }

    #[test]
    fn test_analyze_at_mixed_intents_edge_case() {
        let now = get_fixed_now();

        // "Remember this: recall is important"
        // Matches "remember this" -> Remember (First rule)
        // Matches "recall" -> Recall (Second rule)
        // Should pick Remember because it's first in INTENT_RULES.
        let res = analyze_at("Remember this: recall is important", now).unwrap();
        assert_eq!(res.intent, QueryIntent::Remember);
    }
}
