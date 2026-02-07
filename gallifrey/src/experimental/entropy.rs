use crate::stores::KnowledgeStore;
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Metrics representing the temporal stability of the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Total number of entity versions analyzed.
    pub total_versions: usize,
    /// Number of "Retcons" (facts recorded after they became valid).
    pub retcons: usize,
    /// Number of "Prophecies" (facts recorded before they became valid).
    pub prophecies: usize,
    /// Mean temporal drift in seconds (Transaction Time - Valid Time).
    /// Positive means we are lagging (reactive).
    /// Negative means we are predicting (proactive).
    pub mean_drift_seconds: f64,
    /// Entropy score (0.0 = Stable, 1.0 = Chaotic).
    /// Calculated based on the ratio of retcons/prophecies to total versions.
    pub entropy_score: f64,
}

/// A gauge for measuring the temporal entropy of the Knowledge Store.
#[derive(Debug)]
pub struct EntropyGauge {
    /// Threshold for considering a time difference as significant drift.
    drift_threshold: Duration,
}

impl EntropyGauge {
    /// Create a new Entropy Gauge with a default threshold (5 seconds).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            drift_threshold: Duration::seconds(5),
        }
    }

    /// Set a custom drift threshold.
    #[must_use]
    pub const fn with_threshold(mut self, threshold: Duration) -> Self {
        self.drift_threshold = threshold;
        self
    }

    /// Measure the entropy of the Knowledge Store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store lock is poisoned.
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self, store: &KnowledgeStore) -> crate::error::GallifreyResult<SystemEntropy> {
        let mut total_versions = 0;
        let mut retcons = 0;
        let mut prophecies = 0;
        let mut total_drift_ms = 0i64;

        store.scan_history(|history| {
            for entity in history {
                total_versions += 1;

                let valid_start = entity.temporal.valid_time.start;
                let transaction_start = entity.temporal.transaction_time.start;

                let drift = transaction_start - valid_start;
                total_drift_ms += drift.num_milliseconds();

                if drift > self.drift_threshold {
                    retcons += 1;
                } else if drift < -self.drift_threshold {
                    prophecies += 1;
                }
            }
        })?;

        let mean_drift_seconds = if total_versions > 0 {
            (total_drift_ms as f64 / total_versions as f64) / 1000.0
        } else {
            0.0
        };

        let unstable_count = retcons + prophecies;
        let entropy_score = if total_versions > 0 {
            (unstable_count as f64 / total_versions as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        Ok(SystemEntropy {
            total_versions,
            retcons,
            prophecies,
            mean_drift_seconds,
            entropy_score,
        })
    }
}

impl Default for EntropyGauge {
    fn default() -> Self {
        Self::new()
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

    fn create_entity(valid_start: chrono::DateTime<Utc>, trans_start: chrono::DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(valid_start),
                transaction_time: TimeRange::starting_at(trans_start),
            },
            source: None,
        }
    }

    #[test]
    fn test_entropy_calculation() {
        let store = KnowledgeStore::new();
        let now = Utc::now();
        let hour = Duration::hours(1);

        // 1. Stable: Valid = Transaction (roughly)
        let stable = create_entity(now, now);
        store.insert_entity(stable).unwrap();

        // 2. Retcon: Transaction (Now) > Valid (Last Hour)
        // We learned about it late.
        let retcon = create_entity(now - hour, now);
        store.insert_entity(retcon).unwrap();

        // 3. Prophecy: Valid (Next Hour) > Transaction (Now)
        // We predicted it.
        let prophecy = create_entity(now + hour, now);
        store.insert_entity(prophecy).unwrap();

        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&store).unwrap();

        assert_eq!(entropy.total_versions, 3);
        assert_eq!(entropy.retcons, 1, "Should identify 1 retcon");
        assert_eq!(entropy.prophecies, 1, "Should identify 1 prophecy");

        // Retcon drift: +3600s
        // Prophecy drift: -3600s
        // Stable drift: 0s
        // Mean should be roughly 0
        assert!(entropy.mean_drift_seconds.abs() < 1.0);

        // Score: 2 unstable / 3 total = 0.666...
        assert!(entropy.entropy_score > 0.6 && entropy.entropy_score < 0.7);
    }

    #[test]
    fn test_empty_store() {
        let store = KnowledgeStore::new();
        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&store).unwrap();

        assert_eq!(entropy.total_versions, 0);
        assert_eq!(entropy.entropy_score, 0.0);
    }
}
