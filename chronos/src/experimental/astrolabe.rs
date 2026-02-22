//! The Astrolabe 🧭
//!
//! A semantic navigation tool for the Knowledge Graph.
//! It finds paths between entities not just by shortest hop count,
//! but by minimizing semantic distance (embedding similarity).

use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;

/// The Astrolabe navigator.
#[derive(Debug)]
pub struct Astrolabe {
    gallifrey: Arc<Gallifrey>,
}

#[derive(Debug, Clone, PartialEq)]
struct State {
    cost: f32,
    node: EntityId,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering because BinaryHeap is a max-heap, and we want smallest cost
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type Graph = (HashMap<EntityId, Vec<EntityId>>, HashMap<EntityId, Entity>);

impl Astrolabe {
    /// Create a new Astrolabe instance.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Navigate from one entity to another.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or no path exists.
    pub fn navigate(&self, start_name: &str, end_name: &str) -> Result<Vec<String>> {
        let start_node = self.find_entity(start_name)?;
        let end_node = self.find_entity(end_name)?;

        // Build adjacency list (this is expensive, but for experimental use it's fine)
        let (adjacency, id_to_entity) = self.build_graph()?;

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<EntityId, EntityId> = HashMap::new();
        let mut g_score: HashMap<EntityId, f32> = HashMap::new();

        g_score.insert(start_node.id, 0.0);
        open_set.push(State {
            cost: Self::heuristic(&start_node, &end_node),
            node: start_node.id,
        });

        while let Some(State { cost: _, node }) = open_set.pop() {
            if node == end_node.id {
                return Ok(Self::reconstruct_path(&came_from, node, &id_to_entity));
            }

            if let Some(neighbors) = adjacency.get(&node) {
                for neighbor_id in neighbors {
                    // Cost of moving is 1.0 (hop count) but could be weighted by relationship type
                    let tentative_g_score = g_score.get(&node).unwrap_or(&f32::INFINITY) + 1.0;

                    if tentative_g_score < *g_score.get(neighbor_id).unwrap_or(&f32::INFINITY) {
                        came_from.insert(*neighbor_id, node);
                        g_score.insert(*neighbor_id, tentative_g_score);

                        let neighbor_entity = id_to_entity
                            .get(neighbor_id)
                            .ok_or_else(|| anyhow!("Entity missing from map"))?;
                        let f_score =
                            tentative_g_score + Self::heuristic(neighbor_entity, &end_node);

                        open_set.push(State {
                            cost: f_score,
                            node: *neighbor_id,
                        });
                    }
                }
            }
        }

        Err(anyhow!(
            "No path found from '{start_name}' to '{end_name}'"
        ))
    }

    fn reconstruct_path(
        came_from: &HashMap<EntityId, EntityId>,
        mut current: EntityId,
        id_to_entity: &HashMap<EntityId, Entity>,
    ) -> Vec<String> {
        let mut path = vec![current];
        while let Some(&prev) = came_from.get(&current) {
            current = prev;
            path.push(current);
        }
        path.reverse();

        path.iter()
            .filter_map(|id| id_to_entity.get(id).map(|e| e.name.clone()))
            .collect()
    }

    fn heuristic(a: &Entity, b: &Entity) -> f32 {
        // Heuristic: 1.0 - Cosine Similarity
        // If similarity is 1.0 (identical), heuristic is 0.
        // If similarity is 0.0 (orthogonal), heuristic is 1.0.
        // If similarity is -1.0 (opposite), heuristic is 2.0.
        // We want to favor nodes semantically closer to the target.
        match (&a.embedding, &b.embedding) {
            (Some(emb_a), Some(emb_b)) => 1.0 - cosine_similarity(emb_a, emb_b),
            _ => 1.0, // Default cost if no embedding
        }
    }

    fn find_entity(&self, name: &str) -> Result<Entity> {
        let knowledge = self.gallifrey.knowledge();
        let mut found = None;

        knowledge.scan_history(|history| {
            if found.is_some() {
                return;
            }
            if let Some(e) = history.iter().find(|e| {
                e.name.eq_ignore_ascii_case(name) && e.temporal.is_current()
            }) {
                found = Some(e.clone());
            }
        })?;

        found.ok_or_else(|| anyhow!("Entity '{name}' not found"))
    }

    fn build_graph(&self) -> Result<Graph> {
        let knowledge = self.gallifrey.knowledge();
        let mut adjacency: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
        let mut id_to_entity: HashMap<EntityId, Entity> = HashMap::new();

        // Populate entities map
        knowledge.scan_history(|history| {
            // Take the latest version of each entity
            if let Some(e) = history.iter().find(|e| e.temporal.is_current()) {
                id_to_entity.insert(e.id, e.clone());
            }
        })?;

        // Populate adjacency list
        knowledge.scan_relationships(|rels| {
            for rel in rels {
                // Ensure both source and target exist in our current view
                if id_to_entity.contains_key(&rel.source) && id_to_entity.contains_key(&rel.target)
                {
                    adjacency.entry(rel.source).or_default().push(rel.target);
                    // Assume directed for now. If we want undirected, uncomment below:
                    // adjacency.entry(rel.target).or_default().push(rel.source);
                }
            }
        })?;

        Ok((adjacency, id_to_entity))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Relationship;

    fn create_entity(name: &str, embedding: Option<Vec<f32>>) -> Entity {
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

    fn create_rel(source: EntityId, target: EntityId) -> Relationship {
        Relationship {
            id: EntityId::new(),
            relationship_type: "LINK".to_string(),
            source,
            target,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 1e-6);
    }

    #[test]
    fn test_pathfinding() {
        let gallifrey = Arc::new(Gallifrey::new());
        let store = gallifrey.knowledge();

        // A -> B -> C
        // A is close to C semantically, but no direct link.

        let a = create_entity("A", Some(vec![1.0, 0.0, 0.0]));
        let b = create_entity("B", Some(vec![0.0, 1.0, 0.0]));
        let c = create_entity("C", Some(vec![1.0, 0.1, 0.0])); // Close to A

        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;

        store.insert_entity(a).unwrap();
        store.insert_entity(b).unwrap();
        store.insert_entity(c).unwrap();

        store.insert_relationship(create_rel(id_a, id_b)).unwrap();
        store.insert_relationship(create_rel(id_b, id_c)).unwrap();

        let astrolabe = Astrolabe::new(gallifrey);
        let path = astrolabe.navigate("A", "C").unwrap();

        assert_eq!(path, vec!["A", "B", "C"]);
    }
}
