//! # Entropy Gauge
//!
//! Measures the "Temporal Entropy" of a system based on the divergence between
//! Valid Time and Transaction Time.
//!
//! High entropy indicates a system that is constantly "rewriting history" (Retcons)
//! or "predicting the future" (Prophecies), suggesting instability or high agent activity.

use tardis_common::domain::Entity;

/// Represents the stability of the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemEntropy {
    /// Total temporal drift in milliseconds (absolute sum).
    pub total_entropy: f64,
    /// Number of "Retcons" (Transaction Time > Valid Time).
    /// History was written AFTER it happened.
    pub retcon_count: usize,
    /// Number of "Prophecies" (Valid Time > Transaction Time).
    /// Future was written BEFORE it happened.
    pub prophecy_count: usize,
    /// Average drift per entity in milliseconds.
    pub average_drift_ms: f64,
    /// Normalized stability score (0.0 = Chaos, 1.0 = Stable).
    /// Derived from entropy and counts.
    pub stability_score: f64,
}

/// The Entropy Gauge analyzer.
#[derive(Debug, Default)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Create a new `EntropyGauge`.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Analyze the history of entities to calculate system entropy.
    ///
    /// # Arguments
    /// * `history` - A slice of entities to analyze.
    /// * `threshold_ms` - The threshold in milliseconds to consider a drift significant.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn analyze(&self, history: &[Entity], threshold_ms: i64) -> SystemEntropy {
        let mut total_entropy = 0.0;
        let mut retcon_count = 0;
        let mut prophecy_count = 0;
        let count = history.len();

        if count == 0 {
            return SystemEntropy {
                total_entropy: 0.0,
                retcon_count: 0,
                prophecy_count: 0,
                average_drift_ms: 0.0,
                stability_score: 1.0,
            };
        }

        for entity in history {
            let vt_start = entity.temporal.valid_time.start;
            let tt_start = entity.temporal.transaction_time.start;

            let drift = (vt_start - tt_start).num_milliseconds();
            let abs_drift = drift.abs() as f64;

            total_entropy += abs_drift;

            if drift.abs() >= threshold_ms {
                if tt_start > vt_start {
                    // Recorded after it happened -> Retcon
                    retcon_count += 1;
                } else if vt_start > tt_start {
                    // Valid in future of recording -> Prophecy
                    prophecy_count += 1;
                }
            }
        }

        let average_drift_ms = total_entropy / count as f64;

        // Stability score: inverse sigmoid of average drift
        // 0 drift -> 1.0
        // High drift -> 0.0
        // Using a simple decay function: 1 / (1 + log10(avg_drift + 1))
        let stability_score = 1.0 / (1.0 + (average_drift_ms + 1.0).log10());

        SystemEntropy {
            total_entropy,
            retcon_count,
            prophecy_count,
            average_drift_ms,
            stability_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(valid_start: DateTime<Utc>, trans_start: DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: valid_start,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: trans_start,
                    end: None,
                },
            },
            source: None,
        }
    }

    #[test]
    fn test_perfect_stability() {
        let now = Utc::now();
        let e1 = create_entity(now, now);
        let e2 = create_entity(now, now);
        let history = vec![e1, e2];

        let gauge = EntropyGauge::new();
        let entropy = gauge.analyze(&history, 100);

        assert!((entropy.total_entropy - 0.0).abs() < f64::EPSILON);
        assert_eq!(entropy.retcon_count, 0);
        assert_eq!(entropy.prophecy_count, 0);
        assert!((entropy.stability_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retcons() {
        let now = Utc::now();
        // Valid 1 hour ago, Recorded Now (Retcon)
        let e1 = create_entity(now - Duration::hours(1), now);
        let history = vec![e1];

        let gauge = EntropyGauge::new();
        let entropy = gauge.analyze(&history, 100);

        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 0);
        assert!(entropy.stability_score < 1.0);
    }

    #[test]
    fn test_prophecies() {
        let now = Utc::now();
        // Valid 1 hour in future, Recorded Now (Prophecy)
        let e1 = create_entity(now + Duration::hours(1), now);
        let history = vec![e1];

        let gauge = EntropyGauge::new();
        let entropy = gauge.analyze(&history, 100);

        assert_eq!(entropy.retcon_count, 0);
        assert_eq!(entropy.prophecy_count, 1);
        assert!(entropy.stability_score < 1.0);
    }
}
