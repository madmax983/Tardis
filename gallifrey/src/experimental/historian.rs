//! # The Historian
//!
//! Analyzes the bi-temporal history of entities to identify temporal anomalies.
//!
//! ## Anomalies
//! - **Retcon**: When we learn something *after* it happened (Transaction Time > Valid Time).
//! - **Prophecy**: When we record something *before* it happens (Valid Time > Transaction Time).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tardis_common::domain::Entity;
use tardis_common::id::EntityId;

/// A temporal anomaly detected in an entity's history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemporalAnomalyType {
    /// We learned this fact after it became true in the real world.
    /// "Revisionist History"
    Retcon,
    /// We recorded this fact before it became true in the real world.
    /// "Prediction"
    Prophecy,
}

/// A specific instance of an anomaly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnomaly {
    /// The ID of the entity where the anomaly was found.
    pub entity_id: EntityId,
    /// The type of anomaly.
    pub anomaly_type: TemporalAnomalyType,
    /// The time difference (magnitude of the anomaly).
    /// Always positive.
    pub magnitude: Duration,
    /// When the fact was valid.
    pub valid_time: DateTime<Utc>,
    /// When the fact was recorded.
    pub transaction_time: DateTime<Utc>,
}

/// A report of all anomalies found in a history scan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryReport {
    /// Detected retcons.
    pub retcons: Vec<TemporalAnomaly>,
    /// Detected prophecies.
    pub prophecies: Vec<TemporalAnomaly>,
    /// Total entities scanned.
    pub scanned_count: usize,
}

/// The Historian engine.
#[derive(Debug, Clone)]
pub struct Historian {
    /// The threshold for considering a time difference an anomaly.
    /// Default is 100ms to account for system latency.
    threshold: Duration,
}

impl Historian {
    /// Create a new Historian with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            threshold: Duration::milliseconds(100),
        }
    }

    /// Set a custom threshold.
    #[must_use]
    pub const fn with_threshold(mut self, threshold: Duration) -> Self {
        self.threshold = threshold;
        self
    }

    /// Analyze a list of entities for temporal anomalies.
    #[must_use]
    pub fn analyze(&self, entities: &[Entity]) -> HistoryReport {
        let mut report = HistoryReport {
            scanned_count: entities.len(),
            ..Default::default()
        };

        for entity in entities {
            let valid_start = entity.temporal.valid_time.start;
            let transaction_start = entity.temporal.transaction_time.start;

            let diff = valid_start.signed_duration_since(transaction_start);
            let abs_diff = if diff < Duration::zero() { -diff } else { diff };

            if abs_diff <= self.threshold {
                continue;
            }

            // Valid > Transaction = Prophecy (Future)
            // Transaction > Valid = Retcon (Past)
            // Note: diff = valid - transaction.
            // If diff > 0, Valid > Transaction -> Prophecy.
            // If diff < 0, Valid < Transaction -> Retcon.

            if diff > Duration::zero() {
                report.prophecies.push(TemporalAnomaly {
                    entity_id: entity.id,
                    anomaly_type: TemporalAnomalyType::Prophecy,
                    magnitude: abs_diff,
                    valid_time: valid_start,
                    transaction_time: transaction_start,
                });
            } else {
                report.retcons.push(TemporalAnomaly {
                    entity_id: entity.id,
                    anomaly_type: TemporalAnomalyType::Retcon,
                    magnitude: abs_diff,
                    valid_time: valid_start,
                    transaction_time: transaction_start,
                });
            }
        }

        report
    }
}

impl Default for Historian {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_common::temporal::TimeRange;

    fn create_entity(valid: DateTime<Utc>, transaction: DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(valid),
                transaction_time: TimeRange::starting_at(transaction),
            },
            source: None,
        }
    }

    #[test]
    fn test_analyze_concurrent() {
        let now = Utc::now();
        let entity = create_entity(now, now); // Simultaneous
        let historian = Historian::new();
        let report = historian.analyze(&[entity]);

        assert_eq!(report.retcons.len(), 0);
        assert_eq!(report.prophecies.len(), 0);
    }

    #[test]
    fn test_analyze_retcon() {
        let now = Utc::now();
        // Valid 1 hour ago, Recorded Now.
        let valid = now - Duration::hours(1);
        let transaction = now;

        let entity = create_entity(valid, transaction);
        let historian = Historian::new();
        let report = historian.analyze(&[entity]);

        assert_eq!(report.retcons.len(), 1);
        assert_eq!(report.prophecies.len(), 0);
        assert_eq!(report.retcons[0].anomaly_type, TemporalAnomalyType::Retcon);
        // Magnitude should be roughly 1 hour
        assert!(report.retcons[0].magnitude >= Duration::hours(1));
    }

    #[test]
    fn test_analyze_prophecy() {
        let now = Utc::now();
        // Valid 1 hour in future, Recorded Now.
        let valid = now + Duration::hours(1);
        let transaction = now;

        let entity = create_entity(valid, transaction);
        let historian = Historian::new();
        let report = historian.analyze(&[entity]);

        assert_eq!(report.retcons.len(), 0);
        assert_eq!(report.prophecies.len(), 1);
        assert_eq!(
            report.prophecies[0].anomaly_type,
            TemporalAnomalyType::Prophecy
        );
        assert!(report.prophecies[0].magnitude >= Duration::hours(1));
    }

    #[test]
    fn test_threshold() {
        let now = Utc::now();
        // 500ms delay
        let valid = now - Duration::milliseconds(500);
        let transaction = now;
        let entity = create_entity(valid, transaction);

        // Default threshold is 100ms, so this should be caught
        let h1 = Historian::new();
        assert_eq!(h1.analyze(std::slice::from_ref(&entity)).retcons.len(), 1);

        // High threshold (1s), should be ignored
        let h2 = Historian::new().with_threshold(Duration::seconds(1));
        assert_eq!(h2.analyze(&[entity]).retcons.len(), 0);
    }
}
