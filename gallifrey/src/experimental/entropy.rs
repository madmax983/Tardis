use crate::error::GallifreyResult;
use crate::stores::KnowledgeStore;
use chrono::Duration;
use tardis_common::id::EntityId;

/// Metrics representing the disorder in the knowledge graph.
#[derive(Debug, Default)]
pub struct SystemEntropy {
    /// A normalized score (0.0 to 1.0) indicating overall system stability.
    /// Higher is more chaotic (frequent updates, retcons).
    pub total_entropy: f64,
    /// Number of "Retcons" detected (corrections to the past).
    pub retcon_count: usize,
    /// List of most volatile entities (ID, volatility score).
    pub volatile_entities: Vec<(EntityId, f64)>,
}

/// A tool to measure temporal entropy in the Knowledge Store.
#[derive(Debug, Clone)]
pub struct EntropyGauge {
    /// The time window to consider for volatility (e.g., look back 1 hour).
    /// If None, considers all history.
    pub time_window: Option<Duration>,
}

impl EntropyGauge {
    /// Create a new `EntropyGauge`.
    #[must_use]
    pub const fn new() -> Self {
        Self { time_window: None }
    }

    /// Set a time window for the gauge.
    #[must_use]
    pub const fn with_window(mut self, window: Duration) -> Self {
        self.time_window = Some(window);
        self
    }

    /// Measure the entropy of the system.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be scanned.
    #[allow(clippy::cast_precision_loss)]
    pub fn measure(&self, store: &KnowledgeStore) -> GallifreyResult<SystemEntropy> {
        let mut report = SystemEntropy::default();
        let mut total_volatility = 0.0;
        let mut total_entities = 0;

        store.scan_history(|history| {
            if history.is_empty() {
                return;
            }
            total_entities += 1;

            // 1. Detect Retcons
            // A retcon is when valid_time.start < transaction_time.start
            // by a significant margin (ignoring small clock skew).
            for version in history {
                let valid_start = version.temporal.valid_time.start;
                let tx_start = version.temporal.transaction_time.start;

                // If valid start is more than 1 second before transaction start
                if tx_start.signed_duration_since(valid_start) > Duration::seconds(1) {
                    report.retcon_count += 1;
                }
            }

            // 2. Calculate Volatility
            // Volatility = Number of versions / Time span of transaction history
            if history.len() > 1 {
                if let (Some(first_ver), Some(last_ver)) = (history.first(), history.last()) {
                    let first_tx = first_ver.temporal.transaction_time.start;
                    let last_tx = last_ver.temporal.transaction_time.start;

                    let seconds = (last_tx - first_tx).num_seconds().abs();
                    let volatility = if seconds > 0 {
                        (history.len() as f64) / (seconds as f64 + 1.0)
                    } else {
                        history.len() as f64 // Infinite bursts are high volatility
                    };

                    // Add to list if volatility is significant
                    if volatility > 0.1 {
                        // Use the ID from the first version (they all share the same ID)
                        report.volatile_entities.push((history[0].id, volatility));
                    }
                    total_volatility += volatility;
                }
            }
        })?;

        // Sort volatile entities descending
        report.volatile_entities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        report.volatile_entities.truncate(10); // Top 10

        // Calculate total entropy
        // Simple heuristic: Retcons are weighted heavily + average volatility
        let avg_volatility = if total_entities > 0 {
            total_volatility / f64::from(total_entities)
        } else {
            0.0
        };

        // Normalize to 0-1 range (arbitrary scaling)
        // 10 retcons or high volatility saturates the gauge
        let raw_score = (report.retcon_count as f64 * 0.1) + avg_volatility;
        report.total_entropy = raw_score.min(1.0);

        Ok(report)
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
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use tardis_common::id::EntityId;
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_entropy_gauge() {
        let store = KnowledgeStore::new();
        let gauge = EntropyGauge::new();

        // 1. Stable Entity
        let stable_id = EntityId::new();
        let stable_entity = Entity {
            id: stable_id,
            entity_type: "Stable".to_string(),
            name: "Stable".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
            // #[cfg(test)]
        };
        store.insert_entity(stable_entity).unwrap();

        // 2. Volatile Entity (Many updates)
        let volatile_id = EntityId::new();
        let volatile_entity = Entity {
            id: volatile_id,
            entity_type: "Volatile".to_string(),
            name: "Volatile".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        store.insert_entity(volatile_entity).unwrap();

        // Update 5 times
        for i in 0..5 {
            let mut updates = HashMap::new();
            updates.insert("count".to_string(), serde_json::json!(i));
            store.update_entity(volatile_id, updates).unwrap();
        }

        // 3. Retcon Entity (Valid time < Transaction time)
        let retcon_id = EntityId::new();
        let now = Utc::now();
        let yesterday = now - Duration::days(1);

        let retcon_entity = Entity {
            id: retcon_id,
            entity_type: "Retcon".to_string(),
            name: "Retcon".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(yesterday),
                transaction_time: TimeRange::starting_at(now),
            },
            source: None,
        };
        store.insert_entity(retcon_entity).unwrap();

        // Measure
        let report = gauge.measure(&store).unwrap();

        println!("Entropy Report: {:?}", report);

        // Assertions
        assert!(report.retcon_count >= 1, "Should detect at least 1 retcon");
        assert!(!report.volatile_entities.is_empty(), "Should detect volatile entities");

        let volatile_entry = report.volatile_entities.iter().find(|(id, _)| *id == volatile_id);
        assert!(volatile_entry.is_some(), "Volatile entity should be in the list");
        assert!(volatile_entry.unwrap().1 > 0.0, "Volatility score should be positive");

        assert!(report.total_entropy > 0.0, "Total entropy should be non-zero");
    }
}
