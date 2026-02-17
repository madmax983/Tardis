//! The Serendipity Engine: Finds hidden connections.
//!
//! "Coincidence is just a connection waiting to be discovered."
//!
//! This module scans the knowledge graph for entities that share significant
//! semantic overlap but are not explicitly connected, suggesting new relationships.

use crate::error::ChronosResult;
use std::collections::{HashSet};
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tardis_gallifrey::domain::{Entity};
use tracing::{info, instrument};

/// A suggested connection between two entities.
#[derive(Debug)]
pub struct Suggestion {
    /// The source entity.
    pub source: Entity,
    /// The target entity.
    pub target: Entity,
    /// The reason for the suggestion.
    pub reason: String,
    /// The confidence score (0.0 to 1.0).
    pub score: f32,
}

/// The Serendipity Engine.
#[derive(Debug)]
pub struct SerendipityEngine {
    gallifrey: Arc<Gallifrey>,
}

impl SerendipityEngine {
    /// Create a new Serendipity Engine.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Find potential connections.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails.
    #[instrument(skip(self))]
    pub fn suggest(&self) -> ChronosResult<Vec<Suggestion>> {
        info!("Serendipity: Scanning for hidden connections...");

        let mut entities = Vec::new();
        self.gallifrey.knowledge().scan_history(|history| {
            if let Some(latest) = history.last() {
                // Filter out system entities or very generic ones
                if latest.entity_type != "System" && !latest.properties.is_empty() {
                    entities.push(latest.clone());
                }
            }
        }).map_err(|e| crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let mut existing_connections = HashSet::new();
        self.gallifrey.knowledge().scan_relationships(|rels| {
            for r in rels {
                // Store undirected edge (min, max) to check existence
                // EntityId doesn't implement Ord, so compare UUIDs
                let (a, b) = if r.source.as_uuid() < r.target.as_uuid() {
                    (r.source, r.target)
                } else {
                    (r.target, r.source)
                };
                existing_connections.insert((a, b));
            }
        }).map_err(|e| crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let mut suggestions = Vec::new();

        // Compare all pairs
        for (i, e1) in entities.iter().enumerate() {
            for e2 in entities.iter().skip(i + 1) {
                // Skip if already connected
                let (a, b) = if e1.id.as_uuid() < e2.id.as_uuid() {
                    (e1.id, e2.id)
                } else {
                    (e2.id, e1.id)
                };

                if existing_connections.contains(&(a, b)) {
                    continue;
                }

                // Compute similarity
                let (score, reason) = self.compute_similarity(e1, e2);
                if score > 0.3 { // Threshold
                    suggestions.push(Suggestion {
                        source: e1.clone(),
                        target: e2.clone(),
                        reason,
                        score,
                    });
                }
            }
        }

        // Sort by score descending
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(suggestions)
    }

    fn compute_similarity(&self, e1: &Entity, e2: &Entity) -> (f32, String) {
        // 1. Property Key Overlap (Jaccard)
        let keys1: HashSet<_> = e1.properties.keys().collect();
        let keys2: HashSet<_> = e2.properties.keys().collect();

        if keys1.is_empty() && keys2.is_empty() {
            return (0.0, String::new());
        }

        let common_keys: Vec<_> = keys1.intersection(&keys2).collect();
        let union_count = keys1.union(&keys2).count();

        let key_similarity = if union_count > 0 {
             common_keys.len() as f32 / union_count as f32
        } else {
            0.0
        };

        // 2. Property Value Overlap (Exact match for now)
        let mut common_values = 0;
        let mut total_values = 0;

        for k in &common_keys {
            // k is &&String from intersection -> collect -> Vec<&&String> -> iter -> &&&String
            // **k is &String
            if let (Some(v1), Some(v2)) = (e1.properties.get(**k), e2.properties.get(**k)) {
                // Simple string comparison for now
                if v1.to_string() == v2.to_string() {
                    common_values += 1;
                }
                total_values += 1;
            }
        }

        let value_similarity = if total_values > 0 {
            common_values as f32 / total_values as f32
        } else {
            0.0
        };

        // Weighted Score
        let score = (key_similarity * 0.4) + (value_similarity * 0.6);

        if score > 0.3 {
            let mut reasons = Vec::new();
            if !common_keys.is_empty() {
                reasons.push(format!("shared properties: {:?}", common_keys.iter().take(3).map(|k| k.as_str()).collect::<Vec<_>>()));
            }
            if common_values > 0 {
                reasons.push(format!("{} matching values", common_values));
            }
            return (score, reasons.join(", "));
        }

        (0.0, String::new())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use serde_json::json;

    fn create_test_entity(name: &str, props: HashMap<String, serde_json::Value>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: props,
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[test]
    fn test_serendipity_finds_match() {
        let gallifrey = Arc::new(Gallifrey::new());

        // Create two similar entities
        let mut props1 = HashMap::new();
        props1.insert("department".to_string(), json!("Engineering"));
        props1.insert("location".to_string(), json!("Building A"));

        let mut props2 = HashMap::new();
        props2.insert("department".to_string(), json!("Engineering")); // Match
        props2.insert("location".to_string(), json!("Building B")); // Mismatch
        props2.insert("role".to_string(), json!("Manager"));

        let e1 = create_test_entity("Alice", props1);
        let e2 = create_test_entity("Bob", props2);

        gallifrey.knowledge().insert_entity(e1).unwrap();
        gallifrey.knowledge().insert_entity(e2).unwrap();

        let engine = SerendipityEngine::new(gallifrey);
        let suggestions = engine.suggest().unwrap();

        assert!(!suggestions.is_empty());
        let s = &suggestions[0];
        assert!(s.score > 0.0);
        assert!(s.reason.contains("department"));
    }
}
