//! Entropy Gauge: Measuring the volatility of the timeline.
//!
//! "Chaos is a ladder." - But here, it's a metric.
//!
//! This module quantifies "System Entropy" by analyzing the divergence between
//! Valid Time (when things happened) and Transaction Time (when we recorded them).

use crate::Gallifrey;
use chrono::Duration;
use std::sync::Arc;
use tracing::{info, instrument};

/// A report on the system's temporal entropy.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SystemEntropy {
    /// Total number of entities scanned.
    pub total_entities: usize,
    /// Total number of versions across all entities.
    pub total_versions: usize,
    /// Number of versions where Valid Time < Transaction Time (Retcons).
    pub retcon_count: usize,
    /// Number of versions where Valid Time > Transaction Time (Prophecies).
    pub prophecy_count: usize,
    /// The entropy score (0.0 = Stable, 1.0 = Chaotic).
    pub entropy_score: f64,
    /// The maximum drift observed (seconds).
    pub max_drift_seconds: i64,
}

/// The Entropy Gauge service.
#[derive(Debug)]
pub struct EntropyGauge {
    gallifrey: Arc<Gallifrey>,
    drift_threshold: Duration,
}

impl EntropyGauge {
    /// Create a new Entropy Gauge.
    ///
    /// # Arguments
    ///
    /// * `gallifrey` - The Gallifrey instance to monitor.
    /// * `drift_threshold_seconds` - Tolerance for clock skew (default 5s).
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, drift_threshold_seconds: u64) -> Self {
        Self {
            gallifrey,
            #[allow(clippy::cast_possible_wrap)]
            drift_threshold: Duration::seconds(drift_threshold_seconds as i64),
        }
    }

    /// Measure the current system entropy.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge store cannot be accessed.
    #[instrument(skip(self))]
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self) -> crate::GallifreyResult<SystemEntropy> {
        let mut entropy = SystemEntropy::default();
        let threshold = self.drift_threshold.num_milliseconds();

        self.gallifrey.knowledge().scan_history(|history| {
            entropy.total_entities += 1;
            entropy.total_versions += history.len();

            for version in history {
                let valid = version.temporal.valid_time.start;
                let trans = version.temporal.transaction_time.start;

                // Calculate drift in milliseconds
                let drift_ms = (valid - trans).num_milliseconds();
                let abs_drift = drift_ms.abs();

                if abs_drift > entropy.max_drift_seconds * 1000 {
                    entropy.max_drift_seconds = abs_drift / 1000;
                }

                if drift_ms > threshold {
                    // Prophecy: It's valid in the future, but recorded now.
                    entropy.prophecy_count += 1;
                } else if drift_ms < -threshold {
                    // Retcon: It was valid in the past, but recorded now.
                    entropy.retcon_count += 1;
                }
            }
        })?;

        // Calculate score
        if entropy.total_versions > 0 {
            let chaotic_elements = (entropy.retcon_count + entropy.prophecy_count) as f64;
            // Normalize by total versions.
            // 1.0 means every single version is a retcon or prophecy.
            entropy.entropy_score = chaotic_elements / entropy.total_versions as f64;
        }

        info!("System Entropy Measured: {:.4}", entropy.entropy_score);
        Ok(entropy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use chrono::Utc;

    fn create_entity(valid_start: chrono::DateTime<Utc>, trans_start: chrono::DateTime<Utc>) -> tardis_common::domain::Entity {
        tardis_common::domain::Entity {
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
    fn test_entropy_gauge() {
        let gallifrey = Arc::new(Gallifrey::new());
        let gauge = EntropyGauge::new(gallifrey.clone(), 5);

        let now = Utc::now();
        let past = now - Duration::seconds(100);
        let future = now + Duration::seconds(100);

        // 1. Normal Entity (Valid ≈ Transaction)
        // Valid: Now, Trans: Now
        let e1 = create_entity(now, now);
        gallifrey.knowledge().insert_entity(e1).unwrap();

        // 2. Retcon (Valid < Transaction)
        // Valid: Past, Trans: Now
        let e2 = create_entity(past, now);
        gallifrey.knowledge().insert_entity(e2).unwrap();

        // 3. Prophecy (Valid > Transaction)
        // Valid: Future, Trans: Now
        let e3 = create_entity(future, now);
        gallifrey.knowledge().insert_entity(e3).unwrap();

        let report = gauge.measure().unwrap();

        assert_eq!(report.total_entities, 3);
        assert_eq!(report.total_versions, 3);
        assert_eq!(report.retcon_count, 1);
        assert_eq!(report.prophecy_count, 1);

        // Score should be 2/3 ≈ 0.66
        assert!((report.entropy_score - 0.666).abs() < 0.01);

        // Max drift should be around 100s
        assert!(report.max_drift_seconds >= 99);
    }
}
