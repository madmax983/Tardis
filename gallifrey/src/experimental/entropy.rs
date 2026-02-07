//! # The Entropy Gauge
//!
//! "Chaos is just an order we don't understand yet. But we can measure it."
//!
//! This module quantifies the temporal stability of a system or entity by analyzing the divergence between
//! `Transaction Time` (when we knew it) and `Valid Time` (when it happened).
//!
//! High entropy suggests frequent rewriting of history (Retcons) or speculative execution (Prophecies).
//! A stable system has low entropy, meaning records align closely with reality.

#![cfg(feature = "nova")]

use serde::{Deserialize, Serialize};
use tardis_common::domain::Entity;

/// A quantitative measure of temporal stability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemEntropy {
    /// The stability score, from 0.0 (Time War) to 1.0 (Fixed Point).
    pub score: f64,
    /// Number of times history was rewritten (Valid Time < Transaction Time).
    pub retcon_count: usize,
    /// Number of times the future was predicted (Valid Time > Transaction Time).
    pub prophecy_count: usize,
    /// Total number of versions analyzed.
    pub total_versions: usize,
    /// The average temporal drift in seconds (Valid - Transaction).
    /// Negative means Retcon (lag), Positive means Prophecy (lead).
    pub average_drift_seconds: f64,
}

/// The Entropy Gauge calculator.
#[derive(Debug, Default, Clone, Copy)]
pub struct EntropyGauge {
    /// Threshold in seconds for considering a drift significant.
    /// Drifts smaller than this are considered "Normal" system latency.
    pub threshold_seconds: i64,
}

impl EntropyGauge {
    /// Create a new EntropyGauge with a specific threshold.
    #[must_use]
    pub const fn new(threshold_seconds: i64) -> Self {
        Self { threshold_seconds }
    }

    /// Calculate the entropy of an entity's history.
    ///
    /// # Arguments
    ///
    /// * `history` - The list of entity versions.
    ///
    /// # Returns
    ///
    /// A `SystemEntropy` struct containing the metrics.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn calculate(&self, history: &[Entity]) -> SystemEntropy {
        if history.is_empty() {
            return SystemEntropy {
                score: 1.0,
                ..Default::default()
            };
        }

        let mut retcons = 0;
        let mut prophecies = 0;
        let mut total_drift: f64 = 0.0;

        for entity in history {
            let valid_start = entity.temporal.valid_time.start;
            let transaction_start = entity.temporal.transaction_time.start;

            // Drift = Valid - Transaction
            // If Valid is 12:00 and Transaction is 12:05 (Recorded late), Drift is -5 mins.
            // Wait.
            // Valid (Actual event time) - Transaction (Recording time).
            // If Valid < Transaction -> Drift is negative. (Event happened before recording). This is Retcon/Lag.
            // If Valid > Transaction -> Drift is positive. (Event happens after recording). This is Prophecy.

            let drift = (valid_start - transaction_start).num_seconds();
            total_drift += drift as f64;

            if drift < -self.threshold_seconds {
                retcons += 1;
            } else if drift > self.threshold_seconds {
                prophecies += 1;
            }
        }

        let count = history.len();
        let anomalies = retcons + prophecies;

        // Score calculation:
        // 1.0 - (anomalies / total)
        // If 0 anomalies, score is 1.0.
        // If all are anomalies, score is 0.0.
        let raw_score = 1.0 - (anomalies as f64 / count as f64);
        let score = raw_score.clamp(0.0, 1.0);

        SystemEntropy {
            score,
            retcon_count: retcons,
            prophecy_count: prophecies,
            total_versions: count,
            average_drift_seconds: total_drift / count as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(valid_time: DateTime<Utc>, transaction_time: DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(valid_time),
                transaction_time: TimeRange::starting_at(transaction_time),
            },
            source: None,
        }
    }

    #[test]
    fn test_perfect_stability() {
        let now = Utc::now();
        let gauge = EntropyGauge::new(5);

        // Transaction time exactly matches Valid time
        let e1 = create_entity(now, now);
        let e2 = create_entity(
            now + chrono::Duration::seconds(1),
            now + chrono::Duration::seconds(1),
        );

        let entropy = gauge.calculate(&[e1, e2]);

        assert_eq!(entropy.score, 1.0);
        assert_eq!(entropy.retcon_count, 0);
        assert_eq!(entropy.prophecy_count, 0);
        assert_eq!(entropy.average_drift_seconds, 0.0);
    }

    #[test]
    fn test_time_war_chaos() {
        let now = Utc::now();
        let gauge = EntropyGauge::new(5); // 5 seconds threshold

        // Retcon: Valid time was 1 hour ago, recorded now. (Drift = -3600)
        let e1 = create_entity(now - chrono::Duration::hours(1), now);

        // Prophecy: Valid time is 1 hour in future, recorded now. (Drift = +3600)
        let e2 = create_entity(now + chrono::Duration::hours(1), now);

        let entropy = gauge.calculate(&[e1, e2]);

        // Both are anomalies. 2/2 = 1.0 anomaly ratio. Score = 1.0 - 1.0 = 0.0.
        assert_eq!(entropy.score, 0.0);
        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);
        assert_eq!(entropy.total_versions, 2);
        // Avg drift: (-3600 + 3600) / 2 = 0
        assert_eq!(entropy.average_drift_seconds, 0.0);
    }

    #[test]
    fn test_mixed_history() {
        let now = Utc::now();
        let gauge = EntropyGauge::new(5);

        // Normal (within 5s threshold)
        let e1 = create_entity(now - chrono::Duration::seconds(2), now); // Drift -2

        // Retcon (Drift -100)
        let e2 = create_entity(now - chrono::Duration::seconds(100), now);

        // Normal (Drift +2)
        let e3 = create_entity(now + chrono::Duration::seconds(2), now);

        // Prophecy (Drift +100)
        let e4 = create_entity(now + chrono::Duration::seconds(100), now);

        let entropy = gauge.calculate(&[e1, e2, e3, e4]);

        // 2 anomalies (e2, e4) out of 4 total.
        // Score = 1.0 - (2/4) = 0.5.
        assert_eq!(entropy.score, 0.5);
        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);

        // Avg drift: (-2 + -100 + 2 + 100) / 4 = 0
        assert_eq!(entropy.average_drift_seconds, 0.0);
    }

    #[test]
    fn test_drift_calculation() {
        let now = Utc::now();
        let gauge = EntropyGauge::new(5);

        // Valid is 10s BEFORE Transaction. Drift = -10.
        let e1 = create_entity(now - chrono::Duration::seconds(10), now);

        let entropy = gauge.calculate(&[e1]);

        assert_eq!(entropy.average_drift_seconds, -10.0);
        assert_eq!(entropy.retcon_count, 1);
    }
}
