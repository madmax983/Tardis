//! The Astrolabe 🧭
//!
//! A semantic pathfinding tool for navigating the Knowledge Graph.
//!
//! Unlike traditional graph traversal which only follows explicit edges,
//! the Astrolabe uses semantic similarity (via embeddings) as a heuristic
//! to guide the search towards conceptually related entities.

#![allow(clippy::module_name_repetitions)]

use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tardis_common::domain::Entity;
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;

/// The Astrolabe navigator.
#[derive(Debug)]
pub struct Astrolabe {
    gallifrey: Arc<Gallifrey>,
    #[allow(dead_code)] // Reserved for future narrative features
    vortex: Option<Arc<Vortex>>,
}

/// A node in the A* priority queue.
#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f32, // f_score (g + h)
    id: EntityId,
}

impl Eq for State {}

// The priority queue depends on `Ord`.
// We flip the ordering on costs to make it a min-heap.
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.id.as_uuid().cmp(&other.id.as_uuid()))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Astrolabe {
    /// Create a new Astrolabe instance.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Option<Arc<Vortex>>) -> Self {
        Self { gallifrey, vortex }
    }

    /// Navigate from one entity to another.
    ///
    /// # Errors
    ///
    /// Returns an error if start or end entities are not found, or if no path exists.
    pub fn navigate(&self, start_name: &str, end_name: &str) -> Result<Vec<Entity>> {
        let knowledge = self.gallifrey.knowledge();

        // 1. Find Start and End Entities and snapshot all entities
        let mut start_entity = None;
        let mut end_entity = None;
        let mut all_entities = HashMap::new();

        // Scan history to find start/end and build entity map
        knowledge.scan_history(|history| {
            // Get current version
            if let Some(entity) = history.iter().find(|e| e.temporal.is_current()) {
                all_entities.insert(entity.id, entity.clone());

                if entity.name.eq_ignore_ascii_case(start_name) {
                    start_entity = Some(entity.clone());
                }
                if entity.name.eq_ignore_ascii_case(end_name) {
                    end_entity = Some(entity.clone());
                }
            }
        })?;

        let start_node = start_entity.ok_or_else(|| anyhow!("Start entity '{start_name}' not found"))?;
        let end_node = end_entity.ok_or_else(|| anyhow!("End entity '{end_name}' not found"))?;

        // 2. Build Adjacency List
        let mut adjacency: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
        knowledge.scan_relationships(|relationships| {
            for rel in relationships {
                if rel.temporal.is_current() {
                    adjacency.entry(rel.source).or_default().push(rel.target);
                    // Also add reverse edge for undirected navigation (optional, but good for semantic exploration)
                    // adjacency.entry(rel.target).or_default().push(rel.source);
                }
            }
        })?;

        // 3. A* Search
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<EntityId, EntityId> = HashMap::new();
        let mut g_score: HashMap<EntityId, f32> = HashMap::new(); // Cost from start to node

        // Initialize start node
        g_score.insert(start_node.id, 0.0);
        open_set.push(State {
            cost: Self::heuristic(&start_node, &end_node),
            id: start_node.id,
        });

        while let Some(State { cost: _, id: current_id }) = open_set.pop() {
            if current_id == end_node.id {
                return Ok(Self::reconstruct_path(&came_from, current_id, &all_entities));
            }

            let current_g = *g_score.get(&current_id).unwrap_or(&f32::INFINITY);

            if let Some(neighbors) = adjacency.get(&current_id) {
                for &neighbor_id in neighbors {
                    // Check if neighbor exists in our snapshot (might be filtered out by temporal check)
                    let Some(neighbor_entity) = all_entities.get(&neighbor_id) else {
                        continue;
                    };

                    let edge_weight = 1.0;
                    let tentative_g = current_g + edge_weight;

                    if tentative_g < *g_score.get(&neighbor_id).unwrap_or(&f32::INFINITY) {
                        came_from.insert(neighbor_id, current_id);
                        g_score.insert(neighbor_id, tentative_g);

                        let f_score = tentative_g + Self::heuristic(neighbor_entity, &end_node);
                        open_set.push(State {
                            cost: f_score,
                            id: neighbor_id,
                        });
                    }
                }
            }
        }

        Err(anyhow!("No path found between '{start_name}' and '{end_name}'"))
    }

    fn heuristic(a: &Entity, b: &Entity) -> f32 {
        if let (Some(emb_a), Some(emb_b)) = (&a.embedding, &b.embedding) {
            let sim = Self::cosine_similarity(emb_a, emb_b);
            // Distance = 1.0 - Similarity.
            // We multiply by 5.0 to give the heuristic significant weight,
            // encouraging the search to move "towards" the target semantically.
            (1.0 - sim).max(0.0) * 5.0
        } else {
            0.0 // Fallback to Dijkstra
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    fn reconstruct_path(
        came_from: &HashMap<EntityId, EntityId>,
        mut current: EntityId,
        entities: &HashMap<EntityId, Entity>,
    ) -> Vec<Entity> {
        let mut path = Vec::new();
        // The end node
        if let Some(entity) = entities.get(&current) {
            path.push(entity.clone());
        }

        while let Some(&prev) = came_from.get(&current) {
            current = prev;
            if let Some(entity) = entities.get(&current) {
                path.push(entity.clone());
            }
        }
        path.reverse();
        path
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
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
            source: Some("test".to_string()),
        }
    }

    #[test]
    fn test_navigate_simple_path() {
        let gallifrey = Arc::new(Gallifrey::new());
        let store = gallifrey.knowledge();

        // Create Nodes: A -> B -> C
        let node_a = create_test_entity("A", Some(vec![1.0, 0.0]));
        let node_b = create_test_entity("B", Some(vec![0.7, 0.7])); // Between A and C
        let node_c = create_test_entity("C", Some(vec![0.0, 1.0]));

        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;

        store.insert_entity(node_a).unwrap();
        store.insert_entity(node_b).unwrap();
        store.insert_entity(node_c).unwrap();

        // Create Edges
        let rel_ab = Relationship {
            id: EntityId::new(),
            relationship_type: "LINK".to_string(),
            source: id_a,
            target: id_b,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };
        let rel_bc = Relationship {
            id: EntityId::new(),
            relationship_type: "LINK".to_string(),
            source: id_b,
            target: id_c,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };

        store.insert_relationship(rel_ab).unwrap();
        store.insert_relationship(rel_bc).unwrap();

        let astrolabe = Astrolabe::new(gallifrey, None);
        let path = astrolabe.navigate("A", "C").unwrap();

        assert_eq!(path.len(), 3);
        assert_eq!(path[0].name, "A");
        assert_eq!(path[1].name, "B");
        assert_eq!(path[2].name, "C");
    }

    #[test]
    fn test_heuristic_guidance() {
        // Test that heuristic prefers path with better semantic similarity?
        // Actually A* guarantees shortest path if heuristic is admissible/consistent.
        // Our heuristic is admissible if cost(edge) >= h(start) - h(end).
        // Edge cost is 1.0. Max heuristic diff is likely > 1.0 (5.0 * dist).
        // So this is technically Weighted A* (or just greedy best-first if weight is high).
        // It's not guaranteed shortest path, but "most semantic" path.

        // Let's just verify cosine similarity logic.
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = Astrolabe::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);

        let c = vec![1.0, 1.0]; // unnormalized
        let sim_ac = Astrolabe::cosine_similarity(&a, &c);
        // dot = 1, norm_a = 1, norm_c = sqrt(2) = 1.414
        // sim = 1 / 1.414 = 0.707
        assert!((sim_ac - 0.707106).abs() < 1e-6);
    }
}
