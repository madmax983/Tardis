//! Serendipity: The engine of happy accidents.
//!
//! "Innovation is connecting two existing modules that haven't met yet."
//!
//! This module scans the knowledge graph for hidden connections using semantic similarity.

use crate::error::ChronosResult;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// A suggested connection between two entities.
#[derive(Debug, Clone)]
pub struct ConnectionSuggestion {
    /// The source entity.
    pub source: Entity,
    /// The target entity.
    pub target: Entity,
    /// Semantic similarity score (0.0 to 1.0).
    pub similarity: f32,
    /// Reason for the suggestion.
    pub reason: String,
}

/// The Serendipity engine.
#[derive(Debug)]
pub struct SerendipityEngine {
    gallifrey: Arc<Gallifrey>,
    #[allow(dead_code)] // Reserved for future AI reasoning
    vortex: Arc<Vortex>,
}

impl SerendipityEngine {
    /// Create a new Serendipity engine.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>) -> Self {
        Self { gallifrey, vortex }
    }

    /// Find unconnected entities with high semantic similarity.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning the knowledge graph fails.
    #[instrument(skip(self))]
    pub async fn find_connections(
        &self,
        threshold: f32,
    ) -> ChronosResult<Vec<ConnectionSuggestion>> {
        info!("Serendipity: Scanning for hidden connections...");

        let knowledge = self.gallifrey.knowledge();
        let mut embeddings: HashMap<EntityId, (Entity, Vec<f32>)> = HashMap::new();
        let mut existing_connections: HashSet<(EntityId, EntityId)> = HashSet::new();

        // 1. Collect entities with embeddings
        knowledge
            .scan_history(|history| {
                if let Some(latest) = history.last() {
                    // Only consider entities with embeddings
                    if let Some(embedding) = &latest.embedding {
                        // Only consider current entities
                        if latest.temporal.is_current() {
                            embeddings.insert(latest.id, (latest.clone(), embedding.clone()));
                        }
                    }
                }
            })
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        // 2. Collect existing relationships
        knowledge
            .scan_relationships(|rels| {
                for rel in rels {
                    if rel.temporal.is_current() {
                        // Canonicalize edge to undirected for checking
                        let (min, max) = if rel.source.as_uuid() < rel.target.as_uuid() {
                            (rel.source, rel.target)
                        } else {
                            (rel.target, rel.source)
                        };
                        existing_connections.insert((min, max));
                    }
                }
            })
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        let mut suggestions = Vec::new();
        let entities: Vec<_> = embeddings.values().collect();

        // 3. Pairwise comparison
        // Note: O(N^2) complexity. acceptable for Nova (experimental/small scale).
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let (entity_a, emb_a) = entities[i];
                let (entity_b, emb_b) = entities[j];

                // Check if already connected
                let (min, max) = if entity_a.id.as_uuid() < entity_b.id.as_uuid() {
                    (entity_a.id, entity_b.id)
                } else {
                    (entity_b.id, entity_a.id)
                };

                if existing_connections.contains(&(min, max)) {
                    continue;
                }

                let similarity = cosine_similarity(emb_a, emb_b);
                if similarity >= threshold {
                    suggestions.push(ConnectionSuggestion {
                        source: entity_a.clone(),
                        target: entity_b.clone(),
                        similarity,
                        reason: format!("High semantic similarity ({similarity:.2})"),
                    });
                }
            }
        }

        // Sort by similarity descending
        suggestions.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!("Serendipity: Found {} suggestions.", suggestions.len());
        Ok(suggestions)
    }
}

/// Calculate cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Relationship;

    fn create_entity(name: &str, embedding: Vec<f32>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: Some(embedding),
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-6);

        let v3 = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&v1, &v3) - 0.0).abs() < 1e-6);

        let v4 = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v1, &v4) - -1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_find_connections() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // 1. Create 3 entities
        // A and B are similar (both point 'right')
        // C is different (points 'up')
        let a = create_entity("A", vec![0.9, 0.1, 0.0]);
        let b = create_entity("B", vec![0.8, 0.2, 0.0]);
        let c = create_entity("C", vec![0.0, 0.9, 0.1]);

        let id_a = a.id;
        let id_b = b.id;

        gallifrey.insert(a).await.unwrap();
        gallifrey.insert(b).await.unwrap();
        gallifrey.insert(c).await.unwrap();

        let engine = SerendipityEngine::new(gallifrey.clone(), vortex);

        // 2. Find connections (should find A-B)
        let suggestions = engine.find_connections(0.8).await.unwrap();
        assert_eq!(suggestions.len(), 1);
        let s = &suggestions[0];
        // Order is not guaranteed for source/target, check IDs
        let ids = vec![s.source.id, s.target.id];
        assert!(ids.contains(&id_a));
        assert!(ids.contains(&id_b));

        // 3. Add existing relationship A-B
        let rel = Relationship {
            id: EntityId::new(),
            relationship_type: "KNOWS".to_string(),
            source: id_a,
            target: id_b,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };
        gallifrey
            .knowledge()
            .insert_relationship(rel)
            .unwrap();

        // 4. Find connections (should be empty now)
        let suggestions = engine.find_connections(0.8).await.unwrap();
        assert_eq!(suggestions.len(), 0);
    }
}
