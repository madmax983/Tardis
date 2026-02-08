#![allow(clippy::cast_precision_loss)]

use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;

/// System entropy metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEntropy {
    /// Total entropy (sum of absolute drift).
    pub total_entropy: f64,
    /// Number of retcons (Transaction Time > Valid Time).
    pub retcon_count: usize,
    /// Number of prophecies (Valid Time > Transaction Time).
    pub prophecy_count: usize,
    /// Average drift in milliseconds.
    pub average_drift_ms: f64,
}

/// A gauge for measuring system entropy and temporal drift.
#[derive(Debug)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Measure the current system entropy.
    ///
    /// Iterates over all entity histories to calculate the drift between
    /// when facts became true (Valid Time) and when they were recorded (Transaction Time).
    ///
    /// # Errors
    ///
    /// Returns an error if the history scan fails.
    pub fn measure(store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut total_drift_ms = 0.0;
        let mut abs_drift_sum = 0.0;
        let mut retcons = 0;
        let mut prophecies = 0;
        let mut count = 0;

        // Use a threshold to ignore minor processing delays (e.g., 5 seconds)
        let threshold_ms = 5000.0;

        store.scan_history(|history| {
            for entity in history {
                let valid_start = entity.temporal.valid_time.start;
                let trans_start = entity.temporal.transaction_time.start;

                // Drift = Transaction Start - Valid Start
                // If Trans > Valid -> Positive Drift (Late knowledge / Retcon)
                // If Valid > Trans -> Negative Drift (Early knowledge / Prophecy)
                let drift = (trans_start - valid_start).num_milliseconds() as f64;

                if drift.abs() > threshold_ms {
                    if drift > 0.0 {
                        retcons += 1;
                    } else {
                        prophecies += 1;
                    }
                }

                total_drift_ms += drift;
                abs_drift_sum += drift.abs();
                count += 1;
            }
        })?;

        let average_drift_ms = if count > 0 {
            total_drift_ms / f64::from(count)
        } else {
            0.0
        };

        Ok(SystemEntropy {
            total_entropy: abs_drift_sum,
            retcon_count: retcons,
            prophecy_count: prophecies,
            average_drift_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tardis_common::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_entity(drift_seconds: i64) -> Entity {
        let now = Utc::now();
        let valid_start = now;
        let trans_start = now + Duration::seconds(drift_seconds);

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
    fn test_entropy_calculation() {
        let store = KnowledgeStore::new();

        // 1. Perfect sync (0 drift)
        store.insert_entity(create_entity(0)).unwrap();

        // 2. Retcon (Late knowledge, +10s)
        store.insert_entity(create_entity(10)).unwrap();

        // 3. Prophecy (Early knowledge, -10s)
        store.insert_entity(create_entity(-10)).unwrap();

        // 4. Minor drift (within 5s threshold)
        store.insert_entity(create_entity(2)).unwrap();

        let entropy = EntropyGauge::measure(&store).unwrap();

        // Total entities: 4
        // Retcons: 1 (entity 2)
        // Prophecies: 1 (entity 3)
        // Entity 4 is ignored for counts but included in drift stats

        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);

        // Drift sum: |0| + |10000| + |-10000| + |2000| = 22000
        assert!((entropy.total_entropy - 22000.0).abs() < 1.0);

        // Average drift: (0 + 10000 - 10000 + 2000) / 4 = 500
        assert!((entropy.average_drift_ms - 500.0).abs() < 1.0);
    }
}
