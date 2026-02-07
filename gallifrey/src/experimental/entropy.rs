use serde::{Deserialize, Serialize};
use tardis_common::domain::Entity;

/// Metrics describing the stability of the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Number of events where Valid Time < Transaction Time (History rewriting).
    pub retcon_count: usize,
    /// Number of events where Valid Time > Transaction Time (Future prediction).
    pub prophecy_count: usize,
    /// Average drift (Valid - Transaction) in milliseconds.
    pub average_drift_ms: f64,
    /// Variance of the drift (measure of chaos).
    pub drift_variance: f64,
    /// A normalized score (0.0 to 1.0) where 1.0 is perfectly stable.
    pub stability_score: f64,
}

/// A gauge to measure system entropy from entity history.
#[derive(Debug)]
pub struct EntropyGauge {
    /// Threshold in milliseconds to consider a drift significant (not just latency).
    pub drift_threshold_ms: i64,
}

impl Default for EntropyGauge {
    fn default() -> Self {
        Self {
            drift_threshold_ms: 5000, // 5 seconds default
        }
    }
}

impl EntropyGauge {
    /// Create a new `EntropyGauge` with a specific threshold.
    #[must_use]
    pub const fn new(drift_threshold_ms: i64) -> Self {
        Self { drift_threshold_ms }
    }

    /// Measure the entropy of a given history of entities.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self, history: &[Entity]) -> SystemEntropy {
        if history.is_empty() {
            return SystemEntropy {
                retcon_count: 0,
                prophecy_count: 0,
                average_drift_ms: 0.0,
                drift_variance: 0.0,
                stability_score: 1.0,
            };
        }

        let mut retcons = 0;
        let mut prophecies = 0;
        let mut total_drift = 0.0;
        let mut drifts = Vec::with_capacity(history.len());

        for entity in history {
            let valid_start = entity.temporal.valid_time.start;
            let trans_start = entity.temporal.transaction_time.start;

            let drift = (valid_start - trans_start).num_milliseconds() as f64;
            drifts.push(drift);
            total_drift += drift;

            if drift < -(self.drift_threshold_ms as f64) {
                retcons += 1;
            } else if drift > self.drift_threshold_ms as f64 {
                prophecies += 1;
            }
        }

        let avg_drift = total_drift / history.len() as f64;

        // Calculate variance
        let mut sum_sq_diff = 0.0;
        for drift in &drifts {
            sum_sq_diff += (drift - avg_drift).powi(2);
        }
        let variance = sum_sq_diff / history.len() as f64;

        // Calculate stability score
        // Use a sigmoid-like decay based on variance and abnormal event ratio
        let abnormal_ratio = (retcons + prophecies) as f64 / history.len() as f64;

        // Dampen variance impact.
        // We use a simplified decay function: 1 / (1 + sqrt(variance / constant))
        // The constant 1,000,000 implies that a std deviation of 1000ms (1s) reduces score significantly.
        let variance_penalty = 1.0 / (1.0 + (variance / 1_000_000.0).sqrt());
        let event_penalty = 1.0 - abnormal_ratio;

        let stability = variance_penalty * event_penalty;

        SystemEntropy {
            retcon_count: retcons,
            prophecy_count: prophecies,
            average_drift_ms: avg_drift,
            drift_variance: variance,
            stability_score: stability.clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(drift: Duration) -> Entity {
        let now = Utc::now();
        // valid_time = now + drift
        // transaction_time = now
        // drift = valid - transaction
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: now + drift,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: now,
                    end: None,
                },
            },
            source: None,
        }
    }

    #[test]
    fn test_perfect_stability() {
        let history = vec![
            create_entity(Duration::zero()),
            create_entity(Duration::zero()),
            create_entity(Duration::zero()),
        ];

        let gauge = EntropyGauge::default();
        let entropy = gauge.measure(&history);

        assert_eq!(entropy.retcon_count, 0);
        assert_eq!(entropy.prophecy_count, 0);
        assert!(entropy.drift_variance.abs() < 1e-6);
        assert!((entropy.stability_score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_high_retcon_count() {
        let history = vec![
            create_entity(Duration::seconds(-10)), // Retcon
            create_entity(Duration::seconds(-20)), // Retcon
            create_entity(Duration::zero()),       // Normal
        ];

        let gauge = EntropyGauge::default();
        let entropy = gauge.measure(&history);

        assert_eq!(entropy.retcon_count, 2);
        assert_eq!(entropy.prophecy_count, 0);
        assert!(entropy.stability_score < 0.5); // Should be low
    }

    #[test]
    fn test_prophecy_count() {
        let history = vec![
            create_entity(Duration::seconds(10)), // Prophecy
            create_entity(Duration::zero()),
        ];

        let gauge = EntropyGauge::default();
        let entropy = gauge.measure(&history);

        assert_eq!(entropy.prophecy_count, 1);
        assert_eq!(entropy.retcon_count, 0);
    }

    #[test]
    fn test_empty_history() {
        let history = vec![];
        let gauge = EntropyGauge::default();
        let entropy = gauge.measure(&history);
        assert_eq!(entropy.stability_score, 1.0);
    }
}
