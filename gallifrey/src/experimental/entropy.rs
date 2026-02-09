//! System Entropy Gauge.
//!
//! Measures the "chaos" of the system based on temporal drift and historical rewrites.
//!
//! "Entropy" in this context is defined as the divergence between:
//! - **Valid Time**: When something actually happened.
//! - **Transaction Time**: When the system learned about it.
//!
//! High entropy indicates a system that is either:
//! - Lagging behind reality (high positive drift).
//! - Constantly rewriting history (high retcon count).
//! - Speculating wildly about the future (high prophecy count).

use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;

/// Metrics quantifying system stability and temporal coherence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemEntropy {
    /// Total drift in milliseconds (sum of absolute difference between Valid and Transaction start times).
    pub total_drift_ms: i64,
    /// Number of "Retcons" (where Valid Time < Transaction Time).
    /// Indicates learning about the past.
    pub retcon_count: u64,
    /// Number of "Prophecies" (where Valid Time > Transaction Time).
    /// Indicates predicting the future.
    pub prophecy_count: u64,
    /// Average drift per entity version in milliseconds.
    pub average_drift_ms: f64,
}

/// A gauge for measuring system entropy.
#[derive(Debug, Default)]
pub struct EntropyGauge;

impl EntropyGauge {
    /// Create a new entropy gauge.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Measure the current system entropy.
    ///
    /// Iterates over the entire knowledge graph to calculate temporal drift metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be accessed.
    pub fn measure(&self, store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut total_drift_ms = 0i64;
        let mut retcon_count = 0u64;
        let mut prophecy_count = 0u64;
        let mut version_count = 0u64;

        store.scan_history(|history| {
            for entity in history {
                version_count += 1;
                let valid_start = entity.temporal.valid_time.start;
                let transaction_start = entity.temporal.transaction_time.start;

                let drift = (valid_start - transaction_start).num_milliseconds();

                if drift != 0 {
                    total_drift_ms = total_drift_ms.saturating_add(drift.abs());
                }

                // Check for Retcons (Transaction Time > Valid Time)
                // We learned about it AFTER it happened.
                if transaction_start > valid_start {
                    retcon_count += 1;
                }

                // Check for Prophecies (Transaction Time < Valid Time)
                // We recorded it BEFORE it happened.
                if transaction_start < valid_start {
                    prophecy_count += 1;
                }
            }
        })?;

        let average_drift_ms = if version_count > 0 {
            total_drift_ms as f64 / version_count as f64
        } else {
            0.0
        };

        Ok(SystemEntropy {
            total_drift_ms,
            retcon_count,
            prophecy_count,
            average_drift_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::KnowledgeStore;
    use tardis_common::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use chrono::{Utc, Duration};
    use std::collections::HashMap;

    #[test]
    fn test_entropy_calculation() {
        let store = KnowledgeStore::new();
        let now = Utc::now();
        let past = now - Duration::hours(1);
        let future = now + Duration::hours(1);

        // 1. Normal Entity (Valid ~= Transaction)
        // Allowing for small processing delay, effectively 0 drift for this test setup if we match exactly.
        let normal = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Normal".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange { start: now, end: None },
                transaction_time: TimeRange { start: now, end: None },
            },
            source: None,
        };
        store.insert_entity(normal).unwrap();

        // 2. Retcon Entity (Transaction > Valid)
        // Event happened 1 hour ago (Valid), we record it now (Transaction).
        // Valid < Transaction
        let retcon = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Retcon".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange { start: past, end: None },
                transaction_time: TimeRange { start: now, end: None },
            },
            source: None,
        };
        store.insert_entity(retcon).unwrap();

        // 3. Prophecy Entity (Transaction < Valid)
        // Event will happen in 1 hour (Valid), we record it now (Transaction).
        // Valid > Transaction
        let prophecy = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Prophecy".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange { start: future, end: None },
                transaction_time: TimeRange { start: now, end: None },
            },
            source: None,
        };
        store.insert_entity(prophecy).unwrap();

        let gauge = EntropyGauge::new();
        let entropy = gauge.measure(&store).unwrap();

        // Check retcons (Transaction > Valid)
        assert_eq!(entropy.retcon_count, 1, "Expected 1 retcon");

        // Check prophecies (Transaction < Valid)
        assert_eq!(entropy.prophecy_count, 1, "Expected 1 prophecy");

        // Drift calculation:
        // Normal: 0
        // Retcon: abs(past - now) = 1 hour
        // Prophecy: abs(future - now) = 1 hour
        // Total should be approx 2 hours in ms

        let hour_ms = 3600 * 1000;
        let expected_drift = hour_ms * 2;

        // Allow for small time differences
        assert!(
            (entropy.total_drift_ms - expected_drift).abs() < 1000,
            "Total drift {} not close to expected {}", entropy.total_drift_ms, expected_drift
        );

        // Average should be total / 3
        let expected_avg = expected_drift as f64 / 3.0;
        assert!(
            (entropy.average_drift_ms - expected_avg).abs() < 1000.0,
             "Avg drift {} not close to expected {}", entropy.average_drift_ms, expected_avg
        );
    }
}
