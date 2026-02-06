//! Temporal Entropy Gauge.
//!
//! Measures the stability of the system's timeline by quantifying "Retcons" (corrections to the past)
//! and "Prophecies" (predictions of the future).

use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;
use serde::{Deserialize, Serialize};

/// Metrics describing the temporal stability of the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Total unique entities.
    pub entity_count: usize,
    /// Total version records across all entities.
    pub version_count: usize,
    /// Number of records where Transaction Time > Valid Time (Backfill/Correction).
    pub retcon_count: usize,
    /// Number of records where Valid Time > Transaction Time (Prediction).
    pub prophecy_count: usize,
    /// System entropy score (0.0 = Stable, 1.0 = Chaotic).
    /// Calculated as (Retcons + Prophecies) / Version Count.
    pub entropy_score: f32,
}

/// A gauge for measuring system entropy.
#[derive(Debug, Clone)]
pub struct EntropyGauge {
    /// Threshold in milliseconds to consider a time difference an anomaly.
    /// Default is 5000ms (5 seconds) to account for clock skew/latency.
    pub threshold_ms: i64,
}

impl Default for EntropyGauge {
    fn default() -> Self {
        Self { threshold_ms: 5000 }
    }
}

impl EntropyGauge {
    /// Create a new EntropyGauge.
    #[must_use]
    pub fn new(threshold_ms: i64) -> Self {
        Self { threshold_ms }
    }

    /// Measure the current entropy of the Knowledge Store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be scanned.
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self, store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut entity_count = 0;
        let mut version_count = 0;
        let mut retcon_count = 0;
        let mut prophecy_count = 0;

        store.scan_history(|versions| {
            entity_count += 1;
            version_count += versions.len();

            for version in versions {
                let valid_start = version.temporal.valid_time.start;
                let trans_start = version.temporal.transaction_time.start;

                let diff = trans_start
                    .signed_duration_since(valid_start)
                    .num_milliseconds();

                // If Transaction Time is significantly AFTER Valid Time, we learned it late (Retcon)
                if diff > self.threshold_ms {
                    retcon_count += 1;
                }
                // If Valid Time is significantly AFTER Transaction Time, we predicted it (Prophecy)
                else if diff < -self.threshold_ms {
                    prophecy_count += 1;
                }
            }
        })?;

        let total_anomalies = retcon_count + prophecy_count;
        let entropy_score = if version_count > 0 {
            total_anomalies as f32 / version_count as f32
        } else {
            0.0
        };

        Ok(SystemEntropy {
            entity_count,
            version_count,
            retcon_count,
            prophecy_count,
            entropy_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::KnowledgeStore;
    use chrono::Utc;
    use std::collections::HashMap;
    use tardis_common::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    #[test]
    fn test_entropy_measurement() {
        let store = KnowledgeStore::new();

        let now = Utc::now();
        let ten_mins_ago = now - chrono::Duration::minutes(10);
        let five_mins_future = now + chrono::Duration::minutes(5);

        // 1. Normal Entity (Valid from 10m ago, Recorded 10m ago)
        let e1 = Entity {
            id: EntityId::new(),
            entity_type: "Normal".to_string(),
            name: "Normal".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: ten_mins_ago,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: ten_mins_ago,
                    end: None,
                },
            },
            source: None,
        };
        store.insert_entity(e1).unwrap();

        // 2. Retcon Entity (Valid from 10m ago, Recorded NOW) -> Diff = +10m (Retcon)
        let e2 = Entity {
            id: EntityId::new(),
            entity_type: "Retcon".to_string(),
            name: "Retcon".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: ten_mins_ago,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: now,
                    end: None,
                },
            },
            source: None,
        };
        store.insert_entity(e2).unwrap();

        // 3. Prophecy Entity (Valid from 5m Future, Recorded NOW) -> Diff = -5m (Prophecy)
        let e3 = Entity {
            id: EntityId::new(),
            entity_type: "Prophecy".to_string(),
            name: "Prophecy".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: five_mins_future,
                    end: None,
                },
                transaction_time: TimeRange {
                    start: now,
                    end: None,
                },
            },
            source: None,
        };
        store.insert_entity(e3).unwrap();

        // Measure
        let gauge = EntropyGauge::new(1000); // 1 sec threshold
        let entropy = gauge.measure(&store).unwrap();

        assert_eq!(entropy.entity_count, 3);
        assert_eq!(entropy.version_count, 3);
        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);

        // Score = 2 / 3 = 0.666...
        assert!(entropy.entropy_score > 0.6);
        assert!(entropy.entropy_score < 0.7);
    }
}
