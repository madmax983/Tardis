//! The Conscience: Emotional telemetry for the knowledge store.
//!
//! "Does the system feel confused? Overwhelmed? Or just Zen?"
//!
//! This module analyzes the state of the knowledge graph (entropy, activity)
//! to determine a high-level "Mood" for the system.

use crate::error::GallifreyResult;
use crate::experimental::entropy::{EntropyGauge, SystemEntropy};
use crate::stores::KnowledgeStore;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

/// The emotional state of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mood {
    /// Low entropy, low activity. All is calm.
    Zen,
    /// Low entropy, moderate activity. Processing normally.
    Curious,
    /// High entropy. Conflicting timelines (retcons/prophecies).
    Confused,
    /// High activity (recent writes). System is busy.
    Overwhelmed,
    /// High entropy AND high activity. Panic mode.
    Panic,
}

impl Mood {
    /// Get a human-readable description of the mood.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Mood::Zen => "The timeline is stable. The water is still.",
            Mood::Curious => "Active and learning. The timeline is flowing.",
            Mood::Confused => "Reality is fluctuating. Detected retcons or prophecies.",
            Mood::Overwhelmed => "High write volume. The ink is still wet.",
            Mood::Panic => "CHAOS! High drift and high load.",
        }
    }
}

/// A snapshot of the system's conscience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConscience {
    /// The current mood.
    pub mood: Mood,
    /// The underlying entropy metrics.
    pub entropy: SystemEntropy,
    /// Activity score (approximate writes per minute).
    pub activity_score: usize,
}

/// The Conscience analyzer.
#[derive(Debug)]
pub struct Conscience;

impl Conscience {
    /// Assess the current mood of the system.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be scanned.
    #[allow(clippy::cast_precision_loss)]
    pub fn assess(store: &KnowledgeStore) -> GallifreyResult<SystemConscience> {
        // 1. Measure Entropy (Stability)
        let entropy = EntropyGauge::measure_global(store)?;

        // 2. Measure Activity (Recent Writes)
        // We define "recent" as the last 5 minutes of Transaction Time.
        let now = Utc::now();
        let five_mins_ago = now - Duration::minutes(5);
        let mut recent_writes = 0;

        store.scan_history(|history| {
            // Check if any version was written recently
            if history.iter().any(|e| {
                e.temporal.transaction_time.start >= five_mins_ago
                    || e.temporal
                        .transaction_time
                        .end
                        .is_some_and(|end| end >= five_mins_ago)
            }) {
                recent_writes += 1;
            }
        })?;

        // 3. Determine Mood
        // Heuristics:
        // - Stability < 0.5 -> Confused
        // - Activity > 50 (in 5 mins) -> Overwhelmed
        // - Both -> Panic
        // - Else -> Zen/Curious

        let mood = if entropy.stability_score < 0.5 {
            if recent_writes > 50 {
                Mood::Panic
            } else {
                Mood::Confused
            }
        } else if recent_writes > 50 {
            Mood::Overwhelmed
        } else if recent_writes > 5 {
            Mood::Curious
        } else {
            Mood::Zen
        };

        Ok(SystemConscience {
            mood,
            entropy,
            activity_score: recent_writes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallifrey;
    use tardis_common::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_mood_zen() {
        let gallifrey = Gallifrey::new();
        let store = gallifrey.knowledge();

        // Empty store should be Zen
        let conscience = Conscience::assess(&store).unwrap();
        assert_eq!(conscience.mood, Mood::Zen);
    }

    #[tokio::test]
    async fn test_mood_curious() {
        let gallifrey = Gallifrey::new();
        let store = gallifrey.knowledge();

        // Add a few entities (enough to be Curious, not Overwhelmed)
        for _ in 0..10 {
            let entity = Entity {
                id: EntityId::new(),
                entity_type: "Test".to_string(),
                name: "Test".to_string(),
                properties: HashMap::new(),
                embedding: None,
                temporal: BiTemporalInterval::now(),
                source: None,
            };
            store.insert_entity(entity).unwrap();
        }

        let conscience = Conscience::assess(&store).unwrap();
        assert_eq!(conscience.mood, Mood::Curious);
    }

    #[tokio::test]
    async fn test_mood_confused() {
        let gallifrey = Gallifrey::new();
        let store = gallifrey.knowledge();
        let now = Utc::now();
        let hour = Duration::hours(1);

        // Add Retcons (Valid Time < Transaction Time)
        // We need enough drift to lower stability score below 0.5
        // Stability = 1.0 / (1.0 + avg_drift_sec)
        // If avg_drift_sec > 1.0, score < 0.5.
        // So 2 seconds drift is enough.

        for _ in 0..5 {
            let entity = Entity {
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
            store.insert_entity(entity).unwrap();
        }

        let conscience = Conscience::assess(&store).unwrap();
        assert_eq!(conscience.mood, Mood::Confused);
    }
}
