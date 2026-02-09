use serde::{Deserialize, Serialize};
use tardis_common::domain::Entity;

/// Metrics representing the "entropy" or instability of the system based on temporal drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Total drift in milliseconds (sum of absolute difference between Valid and Transaction time).
    pub total_drift_ms: i64,
    /// Average drift per entity in milliseconds.
    pub average_drift_ms: f64,
    /// Number of entities where Valid Time < Transaction Time (rewriting history).
    pub retcon_count: usize,
    /// Number of entities where Valid Time > Transaction Time (predicting future).
    pub prophecy_count: usize,
    /// Number of entities where timestamps are roughly synchronized.
    pub sync_count: usize,
    /// A normalized score (0.0 to 1.0) indicating system stability.
    /// Higher is more stable (less drift).
    pub stability_score: f64,
}

impl SystemEntropy {
    /// Create a new empty entropy report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total_drift_ms: 0,
            average_drift_ms: 0.0,
            retcon_count: 0,
            prophecy_count: 0,
            sync_count: 0,
            stability_score: 1.0,
        }
    }
}

impl Default for SystemEntropy {
    fn default() -> Self {
        Self::new()
    }
}

/// A gauge to measure system entropy from entity history.
#[derive(Debug, Default)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Create a new `EntropyGauge`.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Measure the entropy of a given set of entities.
    ///
    /// # Arguments
    ///
    /// * `entities` - A slice of entities to analyze.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self, entities: &[Entity]) -> SystemEntropy {
        if entities.is_empty() {
            return SystemEntropy::new();
        }

        let mut total_drift_ms = 0;
        let mut retcon_count = 0;
        let mut prophecy_count = 0;
        let mut sync_count = 0;

        for entity in entities {
            let valid_start = entity.temporal.valid_time.start;
            let transaction_start = entity.temporal.transaction_time.start;

            let diff = valid_start.signed_duration_since(transaction_start).num_milliseconds();
            total_drift_ms += diff.abs();

            // We use a small epsilon for sync (e.g., 100ms) to account for processing time
            if diff.abs() < 100 {
                sync_count += 1;
            } else if diff < 0 {
                // Valid Time is BEFORE Transaction Time -> We are recording something from the past
                retcon_count += 1;
            } else {
                // Valid Time is AFTER Transaction Time -> We are predicting something in the future
                prophecy_count += 1;
            }
        }

        let count = entities.len() as f64;
        let average_drift_ms = total_drift_ms as f64 / count;

        // Stability score: 1.0 / (1.0 + log10(1 + avg_drift_sec))
        let avg_drift_sec = average_drift_ms / 1000.0;
        let stability_score = 1.0 / (1.0 + avg_drift_sec.ln_1p());

        SystemEntropy {
            total_drift_ms,
            average_drift_ms,
            retcon_count,
            prophecy_count,
            sync_count,
            stability_score,
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

    fn create_entity(valid_offset_sec: i64, transaction_offset_sec: i64) -> Entity {
        let now = Utc::now();
        let valid_start = now + Duration::seconds(valid_offset_sec);
        let transaction_start = now + Duration::seconds(transaction_offset_sec);

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
                    start: transaction_start,
                    end: None,
                },
            },
            source: None,
        }
    }

    #[test]
    fn test_entropy_empty() {
        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&[]);
        assert_eq!(entropy.total_drift_ms, 0);
        assert!((entropy.stability_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_entropy_sync() {
        // Valid and Transaction at same time (0 offset)
        let entities = vec![create_entity(0, 0), create_entity(0, 0)];
        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&entities);

        assert_eq!(entropy.sync_count, 2);
        assert_eq!(entropy.retcon_count, 0);
        assert_eq!(entropy.prophecy_count, 0);
        assert_eq!(entropy.total_drift_ms, 0);
    }

    #[test]
    fn test_entropy_retcon() {
        // Valid is in past (-3600), Transaction is now (0)
        let entities = vec![create_entity(-3600, 0)];
        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&entities);

        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.sync_count, 0);
        // Drift is abs(-3600 * 1000)
        assert!(entropy.total_drift_ms >= 3_600_000 - 1000);
    }

    #[test]
    fn test_entropy_prophecy() {
        // Valid is in future (+3600), Transaction is now (0)
        let entities = vec![create_entity(3600, 0)];
        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&entities);

        assert_eq!(entropy.prophecy_count, 1);
        assert_eq!(entropy.sync_count, 0);
        assert!(entropy.total_drift_ms >= 3_600_000 - 1000);
    }

    #[test]
    fn test_stability_score() {
        let gauge = EntropyGauge::new();

        // Perfect score
        let perfect = gauge.measure(&[create_entity(0, 0)]);
        assert!((perfect.stability_score - 1.0).abs() < 0.001);

        // High drift
        let chaotic = gauge.measure(&[create_entity(1_000_000, 0)]);
        assert!(chaotic.stability_score < 0.1);
    }
}
