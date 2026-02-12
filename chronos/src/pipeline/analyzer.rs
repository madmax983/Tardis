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
/// Basic recall query:
/// ```
/// use tardis_chronos::pipeline::{analyzer, QueryIntent};
/// use tardis_common::temporal::TemporalReference;
///
/// let analysis = analyzer::analyze("What did we do yesterday?").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::Recall);
/// assert!(!analysis.temporal_refs.is_empty());
/// match &analysis.temporal_refs[0] {
///    TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
///    _ => panic!("Expected relative reference"),
/// }
/// ```
///
/// Storing a memory:
/// ```
/// use tardis_chronos::pipeline::{analyzer, QueryIntent};
///
/// let analysis = analyzer::analyze("Remember that the sky is blue").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::Remember);
/// ```
///
/// Comparing states (temporal diff):
/// ```
/// use tardis_chronos::pipeline::{analyzer, QueryIntent};
///
/// let analysis = analyzer::analyze("How has the user profile changed?").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::TemporalDiff);
/// ```
///
/// System introspection:
/// ```
/// use tardis_chronos::pipeline::{analyzer, QueryIntent};
///
/// let analysis = analyzer::analyze("Take a system snapshot").unwrap();
///
/// assert_eq!(analysis.intent, QueryIntent::SystemQuery);
/// ```
///
/// # Errors
///
/// Returns an error if analysis fails.
pub fn analyze(query: &str) -> ChronosResult<AnalyzedQuery> {
    // Optimization: Hoist to_lowercase() to avoid repeating it in helper methods
    let query_lower = query.to_lowercase();
    let intent = classify_intent(&query_lower);
    let temporal_refs = extract_temporal_refs(&query_lower, Utc::now());
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
fn extract_temporal_refs(
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

    // TODO: Add more sophisticated temporal extraction
    // - NLP-based extraction
    // - Absolute date parsing
    // - Event-based references

    refs
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
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
