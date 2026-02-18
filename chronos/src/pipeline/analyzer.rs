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
use std::fmt::Write;
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
    fn matches<F>(&self, check_contains: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        let has_required = self.required.iter().all(|k| check_contains(k));
        let has_any = self.any.is_empty() || self.any.iter().any(|k| check_contains(k));
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
    let (intent, temporal_refs) = if query.len() < 30 {
        // Optimization: For short queries, avoid allocating a new String.
        // Use a case-insensitive scan instead.
        let check_contains = |k: &str| contains_ignore_ascii_case(query, k);
        (
            classify_intent(&check_contains, query.ends_with('?')),
            extract_temporal_refs(&check_contains, now),
        )
    } else {
        // Optimization: Hoist to_lowercase() to avoid repeating it in helper methods
        let query_lower = query.to_lowercase();
        let check_contains = |k: &str| query_lower.contains(k);
        (
            classify_intent(&check_contains, query_lower.ends_with('?')),
            extract_temporal_refs(&check_contains, now),
        )
    };

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
fn classify_intent<F>(check_contains: F, has_question_mark: bool) -> QueryIntent
where
    F: Fn(&str) -> bool,
{
    for rule in INTENT_RULES {
        if rule.matches(&check_contains) {
            return rule.intent.clone();
        }
    }

    if has_question_mark {
        return QueryIntent::Question;
    }

    QueryIntent::Chat
}

/// Extract temporal references from a query.
fn extract_temporal_refs<F>(check_contains: F, now: DateTime<Utc>) -> Vec<TemporalReference>
where
    F: Fn(&str) -> bool,
{
    let mut refs = Vec::new();

    for rule in TEMPORAL_RULES {
        if check_contains(rule.keyword) {
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

/// Check if haystack contains needle (case-insensitive ASCII).
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle_len = needle.len();
    let haystack_len = haystack.len();

    if needle_len > haystack_len {
        return false;
    }

    let needle_bytes = needle.as_bytes();
    let haystack_bytes = haystack.as_bytes();

    // Naive O(N*M) search.
    // For short strings (<30 chars) and short keywords (<15 chars),
    // this is faster than allocating a new lowercased string.
    for i in 0..=(haystack_len - needle_len) {
        let window = &haystack_bytes[i..i + needle_len];
        // We assume needle is already lowercase (as it comes from rules)
        if window
            .iter()
            .zip(needle_bytes)
            .all(|(h, n)| h.to_ascii_lowercase() == *n)
        {
            return true;
        }
    }
    false
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

    let mut result = String::new();
    for (i, r) in refs.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match r {
            TemporalReference::Relative { text, resolved } => {
                let _ = write!(result, "{} ({})", text, resolved.format("%Y-%m-%d"));
            }
            TemporalReference::Absolute(resolved) => {
                let _ = write!(result, "{}", resolved.format("%Y-%m-%d"));
            }
            TemporalReference::EventBased { event, resolved } => {
                if let Some(res) = resolved {
                    let _ = write!(result, "{} ({})", event, res.format("%Y-%m-%d"));
                } else {
                    result.push_str(event);
                }
            }
            TemporalReference::Implicit => {
                result.push_str("implicit");
            }
        }
    }
    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_intent() {
        // Need to simulate helper behavior for testing
        let classify = |q: &str| {
            let q_lower = q.to_lowercase();
            classify_intent(|k| q_lower.contains(k), q_lower.ends_with('?'))
        };

        assert_eq!(
            classify("Remember that I like pizza"),
            QueryIntent::Remember
        );
        assert_eq!(
            classify("Please remember this conversation"),
            QueryIntent::Remember
        );
        assert_eq!(
            classify("What did we discuss yesterday?"),
            QueryIntent::Recall
        );
        assert_eq!(
            classify("Recall the meeting notes"),
            QueryIntent::Recall
        );
        assert_eq!(
            classify("What was the result?"),
            QueryIntent::Recall
        );
        assert_eq!(
            classify("How has the project changed?"),
            QueryIntent::TemporalDiff
        );
        assert_eq!(
            classify("Show me the system state"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            classify("Take a snapshot"),
            QueryIntent::SystemQuery
        );
        assert_eq!(
            classify("Is this a question?"),
            QueryIntent::Question
        );
        assert_eq!(
            classify("Just chatting"),
            QueryIntent::Chat
        );
    }

    #[test]
    fn test_extract_temporal_refs() {
        let now = Utc::now();
        // Helper
        let extract = |q: &str| {
            let q_lower = q.to_lowercase();
            extract_temporal_refs(|k| q_lower.contains(k), now)
        };

        let refs = extract("What happened yesterday?");
        assert_eq!(refs.len(), 1);

        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
            _ => panic!("Expected relative"),
        }

        let refs = extract("Check last week logs");
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "last week"),
            _ => panic!("Expected relative"),
        }

        let refs = extract("Do it today");
        assert_eq!(refs.len(), 1);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "today"),
            _ => panic!("Expected relative"),
        }

        let refs = extract("Yesterday and today");
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_extract_temporal_refs_resolved() {
        // Use a fixed date for deterministic testing
        // 2024-03-15 12:00:00 UTC
        let now = DateTime::parse_from_rfc3339("2024-03-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let extract = |q: &str| {
             let q_lower = q.to_lowercase();
             extract_temporal_refs(|k| q_lower.contains(k), now)
        };

        // "yesterday" should be 2024-03-14 12:00:00 UTC
        let refs = extract("yesterday");
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-14T12:00:00+00:00"
        );

        // "last week" should be 2024-03-08 12:00:00 UTC
        let refs = extract("last week");
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved().unwrap().to_rfc3339(),
            "2024-03-08T12:00:00+00:00"
        );
    }

    #[test]
    fn test_describe_temporal_context() {
        let now = Utc::now();
        let refs = extract_temporal_refs(|k| "yesterday".contains(k), now);
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

    #[test]
    fn test_contains_ignore_ascii_case() {
        assert!(contains_ignore_ascii_case("Hello World", "world"));
        assert!(contains_ignore_ascii_case("Hello World", "hello"));
        assert!(contains_ignore_ascii_case("Hello", "hello"));
        assert!(!contains_ignore_ascii_case("Hello", "world"));
        assert!(contains_ignore_ascii_case("MixedCASE", "mixedcase"));
        assert!(contains_ignore_ascii_case("EndsWith", "with"));
        assert!(contains_ignore_ascii_case("StartsWith", "starts"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod sentry_tests {
    use super::*;

    #[test]
    fn test_describe_temporal_context_variants() {
        let now = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let refs = vec![
            TemporalReference::Relative {
                text: "yesterday".to_string(),
                resolved: now,
            },
            TemporalReference::Absolute(now),
            TemporalReference::EventBased {
                event: "Big Bang".to_string(),
                resolved: Some(now),
            },
            TemporalReference::EventBased {
                event: "Heat Death".to_string(),
                resolved: None,
            },
            TemporalReference::Implicit,
        ];

        let desc = describe_temporal_context(&refs);

        // Expected format: "yesterday (2024-01-01), 2024-01-01, Big Bang (2024-01-01), Heat Death, implicit"
        assert!(desc.contains("yesterday (2024-01-01)"), "Missing relative");
        assert!(desc.contains("2024-01-01"), "Missing absolute");
        assert!(desc.contains("Big Bang (2024-01-01)"), "Missing resolved event");
        assert!(desc.contains("Heat Death"), "Missing unresolved event");
        assert!(desc.contains("implicit"), "Missing implicit");
    }

    #[test]
    fn test_extract_entities_stub() {
        let entities = extract_entities("The Doctor went to Gallifrey");
        assert!(entities.is_empty(), "extract_entities should be a stub returning empty vector");
    }

    #[test]
    fn test_classify_intent_edge_cases() {
        let cases = vec![
            ("", QueryIntent::Chat),
            ("   ", QueryIntent::Chat),
            ("?", QueryIntent::Question),
            ("!?", QueryIntent::Question),
            ("Remember", QueryIntent::Chat), // "Remember" alone is not enough based on rules
            ("Remember to buy milk", QueryIntent::Chat), // "remember that" is required
            ("Please remember that", QueryIntent::Remember),
            ("recall", QueryIntent::Recall), // "recall" IS in `any` list
            ("snapshot", QueryIntent::SystemQuery), // "snapshot" IS in `any` list
            ("CHANGE", QueryIntent::Chat), // "change" is NOT in rules. Required: "how has" + "changed"
            ("how has it changed", QueryIntent::TemporalDiff),
        ];

        for (input, expected) in cases {
            assert_eq!(
                classify_intent(&input.to_lowercase()),
                expected,
                "Failed for input: '{}'", input
            );
        }
    }

    #[test]
    fn test_extract_temporal_refs_ordering() {
        let now = Utc::now();
        // Rules order: yesterday, last week, today.

        // Input: "today yesterday"
        // "yesterday" matches first (rule order).
        // "today" matches next.
        // Result order: yesterday, today.
        let refs = extract_temporal_refs("today yesterday", now);
        assert_eq!(refs.len(), 2);
        match &refs[0] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "yesterday"),
            _ => panic!("Expected yesterday first"),
        }
        match &refs[1] {
            TemporalReference::Relative { text, .. } => assert_eq!(text, "today"),
            _ => panic!("Expected today second"),
        }
    }
}
