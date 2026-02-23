//! The Astrolabe: Semantic Pathfinder.
//!
//! "Second star to the right, and straight on 'til morning."
//!
//! This module implements A* search over the knowledge graph to find
//! semantic paths between entities, using embedding similarity as a heuristic.

use crate::error::{ChronosError, ChronosResult};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;

/// A segment of a semantic path.
#[derive(Debug, Clone)]
pub struct PathSegment {
    /// The entity at this step.
    pub entity: Entity,
    /// The relationship used to reach this entity (if any).
    pub via: Option<String>,
    /// The cumulative cost to reach this point.
    pub cost: f32,
}

/// The Astrolabe engine.
#[derive(Debug)]
pub struct Astrolabe {
    gallifrey: Arc<Gallifrey>,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    id: EntityId,
    cost: f32,
    priority: f32,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Astrolabe {
    /// Create a new Astrolabe.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Navigate between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or path cannot be found.
    pub fn navigate(&self, start_name: &str, end_name: &str) -> ChronosResult<Vec<PathSegment>> {
        let start_entity = self.find_entity(start_name)?;
        let end_entity = self.find_entity(end_name)?;

        // A* Search
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<EntityId, (EntityId, Option<String>)> = HashMap::new();
        let mut g_score: HashMap<EntityId, f32> = HashMap::new();

        g_score.insert(start_entity.id, 0.0);

        open_set.push(State {
            id: start_entity.id,
            cost: 0.0,
            priority: Self::heuristic(&start_entity, &end_entity),
        });

        // Pre-fetch graph structure (optimization: scan once)
        let adjacency = self.build_adjacency_map()?;
        let entities = self.build_entity_map()?;

        while let Some(State {
            id: current_id,
            cost: current_g,
            ..
        }) = open_set.pop()
        {
            if current_id == end_entity.id {
                return Ok(Self::reconstruct_path(current_id, &came_from, &entities, &g_score));
            }

            // If we found a shorter path already, skip
            if let Some(&score) = g_score.get(&current_id) {
                if current_g > score {
                    continue;
                }
            }

            if let Some(neighbors) = adjacency.get(&current_id) {
                for (neighbor_id, relation_type) in neighbors {
                    if let Some(neighbor_entity) = entities.get(neighbor_id) {
                        // Cost = 1.0 (hop) + Semantic Drag
                        // Semantic Drag = (1.0 - Similarity) * 2.0 (To weight semantics heavily)
                        let semantic_drag =
                            (1.0 - Self::similarity(neighbor_entity, &end_entity)) * 2.0;
                        let tentative_g = current_g + 1.0 + semantic_drag;

                        if tentative_g < *g_score.get(neighbor_id).unwrap_or(&f32::INFINITY) {
                            came_from
                                .insert(*neighbor_id, (current_id, Some(relation_type.clone())));
                            g_score.insert(*neighbor_id, tentative_g);

                            let f_score =
                                tentative_g + Self::heuristic(neighbor_entity, &end_entity);
                            open_set.push(State {
                                id: *neighbor_id,
                                cost: tentative_g,
                                priority: f_score,
                            });
                        }
                    }
                }
            }
        }

        Err(ChronosError::Common(tardis_common::Error::RetrievalFailed(
            "Path not found".to_string(),
        )))
    }

    fn find_entity(&self, name: &str) -> ChronosResult<Entity> {
        let knowledge = self.gallifrey.knowledge();
        let mut found = None;

        knowledge
            .scan_history(|history| {
                if found.is_some() {
                    return;
                }
                // Case-insensitive match, prefer exact match if possible, otherwise first match
                // Actually, just find the first one that matches case-insensitively and is current.
                if let Some(e) = history
                    .iter()
                    .find(|e| e.name.eq_ignore_ascii_case(name) && e.temporal.is_current())
                {
                    found = Some(e.clone());
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        found.ok_or_else(|| {
            ChronosError::Common(tardis_common::Error::EntityNotFound(name.to_string()))
        })
    }

    fn build_adjacency_map(&self) -> ChronosResult<HashMap<EntityId, Vec<(EntityId, String)>>> {
        let mut map: HashMap<EntityId, Vec<(EntityId, String)>> = HashMap::new();
        self.gallifrey
            .knowledge()
            .scan_relationships(|rels| {
                for rel in rels {
                    if rel.temporal.is_current() {
                        map.entry(rel.source)
                            .or_default()
                            .push((rel.target, rel.relationship_type.clone()));
                        // Make it bidirectional for easier navigation?
                        // No, relationships are directed. Navigating backward is "upstream".
                        // Let's stick to directed for now.
                    }
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;
        Ok(map)
    }

    fn build_entity_map(&self) -> ChronosResult<HashMap<EntityId, Entity>> {
        let mut map = HashMap::new();
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                if let Some(e) = history.iter().find(|e| e.temporal.is_current()) {
                    map.insert(e.id, e.clone());
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;
        Ok(map)
    }

    fn heuristic(a: &Entity, b: &Entity) -> f32 {
        (1.0 - Self::similarity(a, b)) * 2.0
    }

    fn similarity(a: &Entity, b: &Entity) -> f32 {
        if let (Some(va), Some(vb)) = (&a.embedding, &b.embedding) {
            cosine_similarity(va, vb)
        } else {
            0.0
        }
    }

    fn reconstruct_path(
        current: EntityId,
        came_from: &HashMap<EntityId, (EntityId, Option<String>)>,
        entities: &HashMap<EntityId, Entity>,
        g_score: &HashMap<EntityId, f32>,
    ) -> Vec<PathSegment> {
        let mut path = Vec::new();
        let mut curr = current;

        // Add end node
        let mut via_to_curr = came_from.get(&curr).and_then(|(_, v)| v.clone());

        if let Some(entity) = entities.get(&curr) {
            path.push(PathSegment {
                entity: entity.clone(),
                via: via_to_curr.clone(),
                cost: *g_score.get(&curr).unwrap_or(&0.0),
            });
        }

        while let Some((prev, _)) = came_from.get(&curr) {
            curr = *prev;
            via_to_curr = came_from.get(&curr).and_then(|(_, v)| v.clone());

            if let Some(entity) = entities.get(&curr) {
                path.push(PathSegment {
                    entity: entity.clone(),
                    via: via_to_curr.clone(),
                    cost: *g_score.get(&curr).unwrap_or(&0.0),
                });
            }
        }

        path.reverse();
        path
    }
}

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
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Relationship;

    fn create_test_entity(name: &str, embedding: Option<Vec<f32>>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_astrolabe_navigate() {
        let gallifrey = Arc::new(Gallifrey::new());

        // Create entities
        // A -> B -> C
        // A is "Start", C is "End"
        let e1 = create_test_entity("Start", Some(vec![1.0, 0.0]));
        let e2 = create_test_entity("Middle", Some(vec![0.5, 0.5]));
        let e3 = create_test_entity("End", Some(vec![0.0, 1.0]));

        let id1 = e1.id;
        let id2 = e2.id;
        let id3 = e3.id;

        gallifrey.insert(e1).await.unwrap();
        gallifrey.insert(e2).await.unwrap();
        gallifrey.insert(e3).await.unwrap();

        // Create relationships
        gallifrey
            .knowledge()
            .insert_relationship(Relationship {
                id: EntityId::new(),
                relationship_type: "LINKS_TO".to_string(),
                source: id1,
                target: id2,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            })
            .unwrap();

        gallifrey
            .knowledge()
            .insert_relationship(Relationship {
                id: EntityId::new(),
                relationship_type: "LINKS_TO".to_string(),
                source: id2,
                target: id3,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            })
            .unwrap();

        let astrolabe = Astrolabe::new(gallifrey);
        let path = astrolabe.navigate("Start", "End").unwrap();

        assert_eq!(path.len(), 3);
        assert_eq!(path[0].entity.name, "Start");
        assert_eq!(path[1].entity.name, "Middle");
        assert_eq!(path[2].entity.name, "End");
    }
}
