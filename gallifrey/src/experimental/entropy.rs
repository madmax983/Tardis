//! # System Entropy Gauge
//!
//! Measures the "chaos" of the system by analyzing physical volatility (process churn, file changes)
//! and epistemic volatility (retcons, prophecies).
//!
//! Higher entropy indicates a less stable system or one that is undergoing significant
//! historical revisionism.

#[cfg(feature = "nova")]
use crate::Gallifrey;
#[cfg(feature = "nova")]
use chrono::Utc;
#[cfg(feature = "nova")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "nova")]
use std::collections::HashSet;
#[cfg(feature = "nova")]
use std::sync::Arc;
#[cfg(feature = "nova")]
use std::time::Duration;

/// System entropy score and breakdown.
#[cfg(feature = "nova")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntropy {
    /// Overall entropy score (0.0 to 1.0).
    pub score: f64,
    /// Volatility due to process creation/termination.
    pub process_volatility: f64,
    /// Instability due to file system changes.
    pub file_instability: f64,
    /// Drift due to historical corrections (Retcons/Prophecies).
    pub epistemic_drift: f64,
}

/// The Entropy Gauge service.
#[cfg(feature = "nova")]
#[derive(Debug)]
pub struct EntropyGauge {
    gallifrey: Arc<Gallifrey>,
}

#[cfg(feature = "nova")]
impl EntropyGauge {
    /// Create a new `EntropyGauge`.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Measure the system entropy over a given time window.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying stores cannot be accessed.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn measure(&self, window: Duration) -> crate::GallifreyResult<SystemEntropy> {
        let now = Utc::now();
        let start_time = now - chrono::Duration::from_std(window).unwrap_or_else(|_| chrono::Duration::seconds(0));

        // 1. Process Volatility
        // Compare process lists of consecutive snapshots within the window.
        let snapshots = self.gallifrey.system_state().list_snapshots()?;
        let recent_snapshots: Vec<_> = snapshots
            .into_iter()
            .filter(|s| s.timestamp >= start_time)
            .collect();

        let process_volatility = if recent_snapshots.len() < 2 {
            0.0
        } else {
            let mut total_distance = 0.0;
            let mut comparisons = 0;

            for window_slice in recent_snapshots.windows(2) {
                let s1 = &window_slice[0];
                let s2 = &window_slice[1];

                let p1: HashSet<_> = s1.state.processes.keys().collect();
                let p2: HashSet<_> = s2.state.processes.keys().collect();

                let intersection = p1.intersection(&p2).count();
                let union = p1.union(&p2).count();

                if union > 0 {
                    let jaccard_index = intersection as f64 / union as f64;
                    total_distance += 1.0 - jaccard_index;
                    comparisons += 1;
                }
            }

            if comparisons > 0 {
                total_distance / f64::from(comparisons)
            } else {
                0.0
            }
        };

        // 2. File Instability
        // Count changes per minute.
        let changes = self.gallifrey.system_state().get_changes(start_time, now)?;
        let minutes = window.as_secs_f64() / 60.0;
        let changes_per_min = if minutes > 0.0 {
            changes.len() as f64 / minutes
        } else {
            0.0
        };
        // Normalize: assume > 60 changes/min is 1.0 chaos (arbitrary heuristic)
        let file_instability = (changes_per_min / 60.0).clamp(0.0, 1.0);

        // 3. Epistemic Drift
        // Scan history for versions recorded in this window.
        // Check for Retcons (Valid < Transaction) and Prophecies (Valid > Transaction).
        let mut total_drift_seconds = 0.0;
        let mut drift_count = 0;

        self.gallifrey.knowledge().scan_history(|versions| {
            for entity in versions {
                let recorded = entity.temporal.transaction_time.start;

                // Only consider things learned/recorded in the window
                if recorded >= start_time && recorded <= now {
                    let valid = entity.temporal.valid_time.start;
                    let diff = (valid - recorded).num_seconds().abs();

                    // Only count significant drift (> 5 seconds) to ignore clock skew
                    if diff > 5 {
                        total_drift_seconds += diff as f64;
                        drift_count += 1;
                    }
                }
            }
        })?;

        // Normalize: Average drift in minutes, clamped.
        // e.g., average drift of 1 hour (60 mins) = 1.0
        let avg_drift_seconds = if drift_count > 0 {
            total_drift_seconds / f64::from(drift_count)
        } else {
            0.0
        };
        let epistemic_drift = (avg_drift_seconds / 3600.0).clamp(0.0, 1.0);

        // 4. Weighted Score
        // Equal weights for now
        let score = (process_volatility + file_instability + epistemic_drift) / 3.0;

        Ok(SystemEntropy {
            score: score.clamp(0.0, 1.0),
            process_volatility,
            file_instability,
            epistemic_drift,
        })
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use tardis_common::domain::{Change, ChangeType, Entity, ProcessState, SnapshotTrigger, SystemState};
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use std::collections::HashMap;

    #[test]
    fn test_empty_entropy() {
        let gallifrey = Arc::new(Gallifrey::new());
        let gauge = EntropyGauge::new(gallifrey);

        let entropy = gauge.measure(Duration::from_secs(60)).unwrap();

        assert_eq!(entropy.score, 0.0);
        assert_eq!(entropy.process_volatility, 0.0);
        assert_eq!(entropy.file_instability, 0.0);
        assert_eq!(entropy.epistemic_drift, 0.0);
    }

    #[test]
    fn test_high_entropy() {
        let gallifrey = Arc::new(Gallifrey::new());
        let gauge = EntropyGauge::new(gallifrey.clone());

        let now = Utc::now();

        // 1. Simulate Process Volatility
        // Snapshot 1: Process 1, 2
        let mut s1 = SystemState {
            processes: HashMap::new(),
            config: HashMap::new(),
            files: HashMap::new(),
        };
        s1.processes.insert(1, ProcessState { pid: 1, name: "init".into(), status: "R".into(), memory_bytes: 100, cpu_percent: 0.1 });
        s1.processes.insert(2, ProcessState { pid: 2, name: "bash".into(), status: "S".into(), memory_bytes: 200, cpu_percent: 0.0 });

        gallifrey.system_state().take_snapshot("snap1", SnapshotTrigger::Scheduled, s1).unwrap();

        // Snapshot 2: Process 1, 3 (2 died, 3 started) -> 50% change
        let mut s2 = SystemState {
            processes: HashMap::new(),
            config: HashMap::new(),
            files: HashMap::new(),
        };
        s2.processes.insert(1, ProcessState { pid: 1, name: "init".into(), status: "R".into(), memory_bytes: 100, cpu_percent: 0.1 });
        s2.processes.insert(3, ProcessState { pid: 3, name: "curl".into(), status: "R".into(), memory_bytes: 300, cpu_percent: 10.0 });

        gallifrey.system_state().take_snapshot("snap2", SnapshotTrigger::Scheduled, s2).unwrap();

        // 2. Simulate File Instability
        // Add 60 changes in the last minute -> 1.0 instability
        for i in 0..60 {
             let change = Change {
                timestamp: now - chrono::Duration::seconds(i),
                path: format!("/tmp/file_{}", i),
                change_type: ChangeType::Create,
                old_value: None,
                new_value: None,
            };
            gallifrey.system_state().record_change(change).unwrap();
        }

        // 3. Simulate Epistemic Drift
        // Add a Retcon: Recorded NOW, but Valid 1 hour ago.
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Retcon".into(),
            name: "Oopsy".into(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange {
                    start: now - chrono::Duration::hours(1), // Happened 1 hour ago
                    end: None
                },
                transaction_time: TimeRange {
                    start: now, // We just found out
                    end: None
                },
            },
            source: None,
        };
        gallifrey.knowledge().insert_entity(entity).unwrap();

        // Measure
        let entropy = gauge.measure(Duration::from_secs(120)).unwrap();

        // Verification
        // Process: {1,2} vs {1,3}. Int=1, Union=3. Jaccard=1/3. Dist=2/3 (~0.66)
        assert!(entropy.process_volatility > 0.6);

        // File: 60 changes / 2 mins = 30 per min. 30/60 = 0.5
        assert!(entropy.file_instability > 0.4 && entropy.file_instability < 0.6);

        // Drift: 1 entity, 3600s drift. Avg = 3600.
        // Normalized: 3600 / 3600 = 1.0.
        assert!(entropy.epistemic_drift > 0.9);

        // Score: (0.66 + 0.5 + 1.0) / 3 ~= 0.72
        assert!(entropy.score > 0.6);
    }
}
