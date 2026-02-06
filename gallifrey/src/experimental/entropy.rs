//! # System Entropy Gauge 🌟
//!
//! Measures the "Timey Wimey" instability of the knowledge graph.
//!
//! Calculates a stability score based on the divergence between Valid Time and Transaction Time.
//! - **Retcons**: Rewriting the past (Transaction Time > Valid Time).
//! - **Prophecies**: Predicting the future (Valid Time > Transaction Time).
//!
//! A high entropy score indicates a system heavily relying on non-linear time operations.

use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Metrics representing the temporal stability of the system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Total number of entity versions analyzed.
    pub version_count: usize,
    /// Number of versions where history was rewritten (Transaction > Valid).
    pub retcon_count: usize,
    /// Number of versions predicting the future (Valid > Transaction).
    pub prophecy_count: usize,
    /// Entropy score (0.0 to 1.0, where 1.0 is maximum instability).
    pub entropy_score: f64,
}

/// A gauge for measuring system entropy.
#[derive(Debug, Clone)]
pub struct EntropyGauge {
    /// The threshold for considering a time difference significant.
    /// Small delays are normal (network latency, processing time).
    clock_skew_threshold: Duration,
}

impl EntropyGauge {
    /// Create a new Entropy Gauge with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            // Default to 100ms to ignore minor system jitter
            clock_skew_threshold: Duration::milliseconds(100),
        }
    }

    /// Set a custom clock skew threshold.
    #[must_use]
    pub fn with_threshold(mut self, threshold: Duration) -> Self {
        self.clock_skew_threshold = threshold;
        self
    }

    /// Measure the current entropy of the knowledge store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be accessed.
    pub fn measure(&self, store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut total = 0;
        let mut retcons = 0;
        let mut prophecies = 0;

        store.scan_history(|versions| {
            for entity in versions {
                total += 1;
                let valid_start = entity.temporal.valid_time.start;
                let trans_start = entity.temporal.transaction_time.start;

                let diff = valid_start.signed_duration_since(trans_start);

                if diff > self.clock_skew_threshold {
                    // Valid is significantly after Transaction -> Prophecy
                    prophecies += 1;
                } else if diff < -self.clock_skew_threshold {
                    // Valid is significantly before Transaction -> Retcon
                    retcons += 1;
                }
            }
        })?;

        let unstable_events = retcons + prophecies;
        let entropy_score = if total == 0 {
            0.0
        } else {
            // Simple ratio for now. Could be logarithmic or weighted in future.
            unstable_events as f64 / total as f64
        };

        Ok(SystemEntropy {
            version_count: total,
            retcon_count: retcons,
            prophecy_count: prophecies,
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
    use crate::stores::{Entity, KnowledgeStore};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

    fn create_dummy_entity(valid_offset: i64, trans_offset: i64) -> Entity {
        let now = Utc::now();
        let valid_start = now + Duration::seconds(valid_offset);
        let trans_start = now + Duration::seconds(trans_offset);

        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
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
    fn test_entropy_measurement() {
        let store = KnowledgeStore::new();

        // 1. Normal event (Valid ~= Transaction)
        store.insert_entity(create_dummy_entity(0, 0)).unwrap();

        // 2. Retcon (Valid = -10s, Transaction = 0s) -> Valid is before Trans
        store.insert_entity(create_dummy_entity(-10, 0)).unwrap();

        // 3. Prophecy (Valid = +10s, Transaction = 0s) -> Valid is after Trans
        store.insert_entity(create_dummy_entity(10, 0)).unwrap();

        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&store).unwrap();

        assert_eq!(entropy.version_count, 3);
        assert_eq!(entropy.retcon_count, 1, "Should have 1 retcon");
        assert_eq!(entropy.prophecy_count, 1, "Should have 1 prophecy");
        assert!(entropy.entropy_score > 0.6 && entropy.entropy_score < 0.7);
    }
}
