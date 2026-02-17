//! Serendipity Engine: Discovering hidden connections.
//!
//! "Coincidence is just the universe's way of being lazy."
//!
//! This module scans the knowledge graph for entities that share high semantic similarity
//! but are not explicitly connected, then uses Vortex to hypothesize a relationship.

use crate::error::ChronosResult;
use std::collections::HashSet;
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::config::InferenceParams;
use tardis_vortex::{ModelHandle, Vortex};
use tracing::{info, instrument, warn};

/// A potential connection discovered by Serendipity.
#[derive(Debug, Clone)]
pub struct SerendipityConnection {
    /// The first entity.
    pub source: Entity,
    /// The second entity.
    pub target: Entity,
    /// The similarity score (0.0 to 1.0).
    pub similarity: f32,
    /// The AI-generated hypothesis for why they are connected.
    pub hypothesis: String,
}

/// The Serendipity engine.
#[derive(Debug)]
pub struct SerendipityEngine {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl SerendipityEngine {
    /// Create a new Serendipity engine.
    #[must_use]
    pub const fn new(
        gallifrey: Arc<Gallifrey>,
        vortex: Arc<Vortex>,
        model: ModelHandle,
    ) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Find serendipitous connections.
    ///
    /// Scans for unconnected entities with high similarity.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails or inference fails.
    #[instrument(skip(self))]
    pub async fn find_connections(&self, threshold: f32) -> ChronosResult<Vec<SerendipityConnection>> {
        info!("Serendipity: Scanning for hidden connections...");

        // 1. Gather all current entities
        let mut entities = Vec::new();
        let knowledge = self.gallifrey.knowledge();

        // Use scan_history to get entities without cloning the whole map if possible,
        // but we need to collect them to iterate pairs.
        // We only want the *latest* version of each entity that is currently valid.
        knowledge.scan_history(|history| {
            if let Some(latest) = history.last() {
                // Check if it's currently valid (simplification: assume last is current for now,
                // but ideally check temporal.active_at(now, now))
                // Also skip System entities to keep it interesting
                if latest.entity_type != "System" && !latest.name.is_empty() {
                    entities.push(latest.clone());
                }
            }
        }).map_err(|e| crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        if entities.len() < 2 {
            return Ok(Vec::new());
        }

        // 2. Build adjacency set of existing relationships to avoid proposing known connections
        let mut connected_pairs = HashSet::new();
        knowledge.scan_relationships(|rels| {
            for rel in rels {
                // Store both directions as undirected connection
                if rel.source.as_uuid() < rel.target.as_uuid() {
                    connected_pairs.insert((rel.source, rel.target));
                } else {
                    connected_pairs.insert((rel.target, rel.source));
                }
            }
        }).map_err(|e| crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // 3. Find candidate pairs
        let mut candidates = Vec::new();
        // Limit to avoid O(N^2) explosion.
        // For MVP, we just take the first 50 entities.
        let limit = 50;
        let pool = if entities.len() > limit { &entities[..limit] } else { &entities };

        for (i, a) in pool.iter().enumerate() {
            for b in pool.iter().skip(i + 1) {
                // Check if already connected
                let pair_key = if a.id.as_uuid() < b.id.as_uuid() {
                    (a.id, b.id)
                } else {
                    (b.id, a.id)
                };
                if connected_pairs.contains(&pair_key) {
                    continue;
                }

                // Compute similarity
                let sim = Self::compute_similarity(a, b);
                if sim >= threshold {
                    candidates.push((a.clone(), b.clone(), sim));
                }
            }
        }

        // Sort by similarity descending
        candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 3 candidates to explain
        let top_candidates = candidates.into_iter().take(3).collect::<Vec<_>>();

        let mut results = Vec::new();

        for (a, b, sim) in top_candidates {
            info!("Serendipity: Analyzing connection between '{}' and '{}' (score: {:.2})", a.name, b.name, sim);

            // Ask Vortex for hypothesis
            let prompt = format!(
                "I found two entities in the database that seem related but are not connected.\n\
                 Entity A: {{ \"name\": \"{}\", \"type\": \"{}\", \"properties\": {:?} }}\n\
                 Entity B: {{ \"name\": \"{}\", \"type\": \"{}\", \"properties\": {:?} }}\n\
                 Task: Explain why these two might be related in 1 sentence. Propose a relationship type (e.g., RELATED_TO, CAUSES, INSPIRES).\n\
                 Hypothesis:",
                a.name, a.entity_type, a.properties,
                b.name, b.entity_type, b.properties
            );

            let params = InferenceParams::default()
                .with_temperature(0.7)
                .with_max_tokens(100);

            match self.vortex.infer(self.model, &prompt, params).await {
                Ok(hypothesis) => {
                    results.push(SerendipityConnection {
                        source: a,
                        target: b,
                        similarity: sim,
                        hypothesis: hypothesis.trim().to_string(),
                    });
                }
                Err(e) => {
                    warn!("Serendipity: Failed to generate hypothesis: {}", e);
                }
            }
        }

        Ok(results)
    }

    /// Compute similarity between two entities.
    /// Uses embedding cosine similarity if available, otherwise Jaccard index of properties.
    fn compute_similarity(a: &Entity, b: &Entity) -> f32 {
        if let (Some(vec_a), Some(vec_b)) = (&a.embedding, &b.embedding) {
            // Cosine similarity
            let dot_product: f32 = vec_a.iter().zip(vec_b).map(|(x, y)| x * y).sum();
            let norm_a: f32 = vec_a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = vec_b.iter().map(|x| x * x).sum::<f32>().sqrt();

            if norm_a > 0.0 && norm_b > 0.0 {
                return dot_product / (norm_a * norm_b);
            }
        }

        // Fallback: Property Key Overlap (Jaccard)
        let keys_a: HashSet<_> = a.properties.keys().collect();
        let keys_b: HashSet<_> = b.properties.keys().collect();

        let intersection = keys_a.intersection(&keys_b).count();
        let union = keys_a.union(&keys_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_vortex::ModelLoadConfig;

    fn create_test_entity(name: &str, props: &[&str]) -> Entity {
        let mut properties = HashMap::new();
        for p in props {
            properties.insert(p.to_string(), serde_json::json!("value"));
        }

        Entity {
            id: EntityId::new(),
            entity_type: "Concept".to_string(),
            name: name.to_string(),
            properties,
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_serendipity_finds_connection() {
        // Setup dependencies
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Insert two similar entities (same properties)
        let a = create_test_entity("Apple", &["fruit", "red", "sweet"]);
        let b = create_test_entity("Cherry", &["fruit", "red", "sweet"]);
        gallifrey.insert(a).await.unwrap();
        gallifrey.insert(b).await.unwrap();

        // Mock Vortex
        let mock_handle = ModelHandle::new(1);
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(mock_handle)));

        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("Both are red sweet fruits.".to_string())
        }));

        // Load dummy model
        let handle = vortex.load_model("dummy", ModelLoadConfig::default()).await.unwrap();

        let engine = SerendipityEngine::new(gallifrey, vortex, handle);

        let connections = engine.find_connections(0.5).await.unwrap();

        assert!(!connections.is_empty());
        let conn = &connections[0];
        assert!(conn.similarity >= 0.99); // Should be 1.0
        assert!(conn.hypothesis.contains("red sweet fruits"));
    }

    #[test]
    fn test_similarity_fallback() {
        let a = create_test_entity("A", &["x", "y"]);
        let b = create_test_entity("B", &["y", "z"]);

        // Intersection: y (1)
        // Union: x, y, z (3)
        // Jaccard: 1/3 = 0.33
        let sim = SerendipityEngine::compute_similarity(&a, &b);
        assert!((sim - 0.333).abs() < 0.01);
    }
}
