//! The Astrolabe 🧭
//!
//! A semantic pathfinder that navigates the knowledge graph using vector embeddings.
//!
//! Unlike traditional graph traversal which only follows explicit edges, the Astrolabe
//! uses semantic similarity as a heuristic to guide the search towards the target concept.

use anyhow::{anyhow, Result};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;

/// The Astrolabe engine.
#[derive(Debug)]
pub struct Astrolabe {
    gallifrey: Arc<Gallifrey>,
    #[allow(dead_code)] // Vortex might be used for future embedding generation
    vortex: Option<Arc<Vortex>>,
}

/// A node in the search frontier.
#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f32,
    node: EntityId,
}

impl Eq for State {}

// The priority queue depends on `Ord`.
// We flip the ordering on costs so the queue becomes a min-heap.
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.node.as_uuid().cmp(&other.node.as_uuid()))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type AdjacencyList = HashMap<EntityId, Vec<EntityId>>;
type EntityMap = HashMap<EntityId, Entity>;

impl Astrolabe {
    /// Create a new Astrolabe instance.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Option<Arc<Vortex>>) -> Self {
        Self { gallifrey, vortex }
    }

    /// Navigate from a start entity to an end entity using semantic A* search.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or no path exists.
    pub fn navigate(&self, start_name: &str, end_name: &str) -> Result<Vec<String>> {
        let (start_id, start_entity) = self.find_entity(start_name)?;
        let (end_id, end_entity) = self.find_entity(end_name)?;

        // 1. Build the graph in memory (Adjacency List + Entity Map)
        let (adj_list, entity_map) = self.build_graph()?;

        // 2. A* Search
        let mut open_set = BinaryHeap::new();
        open_set.push(State {
            cost: 0.0,
            node: start_id,
        });

        let mut came_from: HashMap<EntityId, EntityId> = HashMap::new();
        let mut g_score: HashMap<EntityId, f32> = HashMap::new();
        g_score.insert(start_id, 0.0);

        let mut f_score: HashMap<EntityId, f32> = HashMap::new();
        f_score.insert(
            start_id,
            Self::heuristic(&start_entity, &end_entity),
        );

        let mut visited = HashSet::new();

        while let Some(State { cost: _, node: current }) = open_set.pop() {
            if current == end_id {
                return Self::reconstruct_path(&came_from, current, &entity_map);
            }

            if !visited.insert(current) {
                continue;
            }

            if let Some(neighbors) = adj_list.get(&current) {
                for &neighbor in neighbors {
                    let tentative_g_score = g_score.get(&current).unwrap_or(&f32::INFINITY) + 1.0; // Distance between nodes is 1 hop

                    if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f32::INFINITY) {
                        came_from.insert(neighbor, current);
                        g_score.insert(neighbor, tentative_g_score);

                        let neighbor_entity = entity_map.get(&neighbor).ok_or_else(|| anyhow!("Entity missing from map"))?;
                        let h = Self::heuristic(neighbor_entity, &end_entity);
                        let f = tentative_g_score + h;
                        f_score.insert(neighbor, f);

                        open_set.push(State {
                            cost: f,
                            node: neighbor,
                        });
                    }
                }
            }
        }

        Err(anyhow!("No path found between '{start_name}' and '{end_name}'"))
    }

    fn find_entity(&self, name: &str) -> Result<(EntityId, Entity)> {
        let knowledge = self.gallifrey.knowledge();
        let mut found = None;

        knowledge.scan_history(|history| {
            if found.is_some() {
                return;
            }
            // Find active version
            if let Some(e) = history
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(name) && e.temporal.is_current())
            {
                found = Some((e.id, e.clone()));
            }
        })?;

        found.ok_or_else(|| anyhow!("Entity '{name}' not found"))
    }

    fn build_graph(&self) -> Result<(AdjacencyList, EntityMap)> {
        let mut adj_list: AdjacencyList = HashMap::new();
        let mut entity_map: EntityMap = HashMap::new();

        let knowledge = self.gallifrey.knowledge();

        // Populate entities
        knowledge.scan_history(|history| {
             if let Some(e) = history.iter().find(|e| e.temporal.is_current()) {
                 entity_map.insert(e.id, e.clone());
             }
        })?;

        // Populate edges
        knowledge.scan_relationships(|relationships| {
            for rel in relationships {
                if rel.temporal.is_current() {
                    adj_list.entry(rel.source).or_default().push(rel.target);
                    // Assuming undirected navigation for "semantic association"?
                    // Let's keep it directed for now, as relationships usually are.
                    // But for "association", undirected might be better.
                    // Let's make it bidirectional for semantic navigation.
                    adj_list.entry(rel.target).or_default().push(rel.source);
                }
            }
        })?;

        Ok((adj_list, entity_map))
    }

    fn heuristic(a: &Entity, b: &Entity) -> f32 {
        if let (Some(emb_a), Some(emb_b)) = (&a.embedding, &b.embedding) {
            // Cosine similarity is dot product if normalized.
            // Distance = 1.0 - similarity.
            // Range [0, 2].
            let dot_product: f32 = emb_a.iter().zip(emb_b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f32 = emb_a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = emb_b.iter().map(|x| x * x).sum::<f32>().sqrt();

            if norm_a == 0.0 || norm_b == 0.0 {
                return 1.0; // Max distance if invalid
            }

            let cosine_sim = dot_product / (norm_a * norm_b);
            1.0 - cosine_sim
        } else {
            1.0 // Default penalty if no embeddings
        }
    }

    fn reconstruct_path(
        came_from: &HashMap<EntityId, EntityId>,
        current: EntityId,
        entity_map: &EntityMap,
    ) -> Result<Vec<String>> {
        let mut path = Vec::new();
        let mut curr = current;

        while let Some(prev) = came_from.get(&curr) {
            let entity = entity_map.get(&curr).ok_or_else(|| anyhow!("Entity missing"))?;
            path.push(entity.name.clone());
            curr = *prev;
        }

        // Add start node
        let entity = entity_map.get(&curr).ok_or_else(|| anyhow!("Entity missing"))?;
        path.push(entity.name.clone());

        path.reverse();
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tardis_common::id::EntityId;
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

    fn connect(gallifrey: &Gallifrey, source: EntityId, target: EntityId) {
        let rel = Relationship {
            id: EntityId::new(),
            relationship_type: "LINK".to_string(),
            source,
            target,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };
        gallifrey.knowledge().insert_relationship(rel).unwrap();
    }

    #[tokio::test]
    async fn test_navigate_simple_path() {
        let gallifrey = Arc::new(Gallifrey::new());
        let astrolabe = Astrolabe::new(gallifrey.clone(), None);

        // A -> B -> C
        let a = create_entity("A", Some(vec![1.0, 0.0]));
        let b = create_entity("B", Some(vec![0.7, 0.7])); // closer to C
        let c = create_entity("C", Some(vec![0.0, 1.0]));

        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;

        gallifrey.insert(a).await.unwrap();
        gallifrey.insert(b).await.unwrap();
        gallifrey.insert(c).await.unwrap();

        connect(&gallifrey, id_a, id_b);
        connect(&gallifrey, id_b, id_c);

        let path = astrolabe.navigate("A", "C").unwrap();
        assert_eq!(path, vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn test_navigate_semantic_shortcut() {
        // A -> B -> C (Physical path)
        // A -> D -> C (Semantic shortcut? No, graph topology limits us.)
        // Astrolabe finds the best path ON THE GRAPH based on semantic heuristic.

        // Graph:
        // A --(1)--> B --(1)--> C  (Path 1: A-B-C, Cost 2)
        // A --(1)--> D --(1)--> E --(1)--> C (Path 2: A-D-E-C, Cost 3)

        // Embeddings:
        // A: [1, 0]
        // C: [0, 1]
        // B: [0.9, 0.1] (Very close to A, far from C)
        // D: [0.7, 0.7] (Halfway to C)
        // E: [0.1, 0.9] (Very close to C)

        // Heuristic should guide A -> D -> E -> C because D is semantically closer to C than B is?
        // Wait, A* ensures shortest path if heuristic is admissible.
        // If we want "most semantic path" regardless of length, we'd need to weight edges by semantic distance.
        // Currently edges are weight 1. So it finds shortest hop count, but uses semantics to expand the most promising nodes first.
        // This is standard A*.

        // To make it interesting, let's say edges have semantic weight?
        // For this MVP, unweighted edges + semantic heuristic = faster search for shortest path.
        // It doesn't change the path unless there are multiple shortest paths.

        let gallifrey = Arc::new(Gallifrey::new());
        let astrolabe = Astrolabe::new(gallifrey.clone(), None);

        let a = create_entity("A", Some(vec![1.0, 0.0]));
        let c = create_entity("C", Some(vec![0.0, 1.0]));

        // Path 1: A -> B -> C
        let b = create_entity("B", Some(vec![0.5, 0.5]));

        // Path 2: A -> D -> C
        let d = create_entity("D", Some(vec![0.5, 0.5]));

        gallifrey.insert(a.clone()).await.unwrap();
        gallifrey.insert(b.clone()).await.unwrap();
        gallifrey.insert(c.clone()).await.unwrap();
        gallifrey.insert(d.clone()).await.unwrap();

        connect(&gallifrey, a.id, b.id);
        connect(&gallifrey, b.id, c.id);

        connect(&gallifrey, a.id, d.id);
        connect(&gallifrey, d.id, c.id);

        let path = astrolabe.navigate("A", "C").unwrap();
        // Should find one of them.
        assert!(path.len() == 3);
        assert_eq!(path[0], "A");
        assert_eq!(path[2], "C");
    }
}
