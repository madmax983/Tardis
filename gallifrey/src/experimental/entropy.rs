use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;
use serde::{Deserialize, Serialize};
use crate::domain::Entity;

/// Metrics quantifying the stability of the system's knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Total drift (absolute difference between Valid Time and Transaction Time) in milliseconds.
    pub total_drift_ms: i64,
    /// Average drift per event in milliseconds.
    pub average_drift_ms: f64,
    /// Number of "Retcons" (Valid Time < Transaction Time).
    pub retcon_count: usize,
    /// Number of "Prophecies" (Valid Time > Transaction Time).
    pub prophecy_count: usize,
    /// Number of "Syncs" (Valid Time ≈ Transaction Time).
    pub sync_count: usize,
    /// A normalized score (0.0 to 1.0) indicating system stability.
    /// 1.0 means perfect real-time recording.
    pub stability_score: f64,
}

impl Default for SystemEntropy {
    fn default() -> Self {
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

/// A gauge for measuring system entropy.
#[derive(Debug)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Measure the entropy of a single entity's history.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(history: &[Entity]) -> SystemEntropy {
        if history.is_empty() {
            return SystemEntropy::default();
        }

        let mut total_drift_ms = 0;
        let mut retcon_count = 0;
        let mut prophecy_count = 0;
        let mut sync_count = 0;

        for entity in history {
            let valid_start = entity.temporal.valid_time.start;
            let transaction_start = entity.temporal.transaction_time.start;

            // Drift is valid - transaction
            // If valid is before transaction (negative): Retcon (we learned it late)
            // If valid is after transaction (positive): Prophecy (we predicted it)
            let drift = (valid_start - transaction_start).num_milliseconds();

            total_drift_ms += drift.abs();

            // Threshold for "Sync" (e.g. within 1 second)
            if drift.abs() < 1000 {
                sync_count += 1;
            } else if drift < 0 {
                retcon_count += 1;
            } else {
                prophecy_count += 1;
            }
        }

        let count = history.len();
        let average_drift_ms = total_drift_ms as f64 / count as f64;

        // Stability score: 1.0 / (1.0 + avg_drift_seconds)
        // If avg drift is 0 -> 1.0
        // If avg drift is 1s (1000ms) -> 0.5
        let stability_score = 1.0 / (1.0 + (average_drift_ms / 1000.0));

        SystemEntropy {
            total_drift_ms,
            average_drift_ms,
            retcon_count,
            prophecy_count,
            sync_count,
            stability_score,
        }
    }

    /// Measure the global entropy across the entire knowledge store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be scanned.
    #[allow(clippy::cast_precision_loss)]
    pub fn measure_global(store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut total_drift_ms = 0;
        let mut total_count = 0;
        let mut retcon_count = 0;
        let mut prophecy_count = 0;
        let mut sync_count = 0;

        store.scan_history(|history| {
            let entropy = Self::measure(history);
            total_drift_ms += entropy.total_drift_ms;
            total_count += history.len();
            retcon_count += entropy.retcon_count;
            prophecy_count += entropy.prophecy_count;
            sync_count += entropy.sync_count;
        })?;

        if total_count == 0 {
            return Ok(SystemEntropy::default());
        }

        let average_drift_ms = total_drift_ms as f64 / total_count as f64;
        let stability_score = 1.0 / (1.0 + (average_drift_ms / 1000.0));

        Ok(SystemEntropy {
            total_drift_ms,
            average_drift_ms,
            retcon_count,
            prophecy_count,
            sync_count,
            stability_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallifrey;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    #[test]
    fn test_measure_logic() {
        let now = Utc::now();
        let hour = Duration::hours(1);

        // 1. Sync: Valid ≈ Transaction
        let sync_entity = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Sync".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };

        // 2. Retcon: Valid < Transaction (learned late)
        // Happened 1 hour ago, recorded now
        let retcon_entity = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Retcon".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(now - hour),
                transaction_time: TimeRange::starting_at(now),
            },
            source: None,
        };

        // 3. Prophecy: Valid > Transaction (predicted future)
        // Will happen in 1 hour, recorded now
        let prophecy_entity = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Prophecy".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(now + hour),
                transaction_time: TimeRange::starting_at(now),
            },
            source: None,
        };

        let history = vec![sync_entity, retcon_entity, prophecy_entity];
        let entropy = EntropyGauge::measure(&history);

        assert_eq!(entropy.sync_count, 1);
        assert_eq!(entropy.retcon_count, 1);
        assert_eq!(entropy.prophecy_count, 1);

        // Total drift: 0 + 3600000 + 3600000 = 7,200,000 ms
        assert!(entropy.total_drift_ms >= 7_200_000);
        assert!(entropy.average_drift_ms >= 2_400_000.0);

        // Stability should be low
        assert!(entropy.stability_score < 0.1);
    }

    #[tokio::test]
    async fn test_measure_global() {
        let gallifrey = Gallifrey::new();
        let store = gallifrey.knowledge();

        // Add one entity
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Global".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };

        store.insert_entity(entity).unwrap();

        // Measure global
        let entropy = EntropyGauge::measure_global(&store).unwrap();

        assert_eq!(entropy.sync_count, 1);
        assert_eq!(entropy.total_drift_ms, 0);
        assert!(entropy.stability_score > 0.9);
    }
}
