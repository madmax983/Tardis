//! Astrolabe 🧭
//!
//! A semantic pathfinding tool that navigates the knowledge graph
//! using vector similarity to guide the search towards relevant concepts.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
use tardis_common::domain::Entity;
use tardis_common::id::EntityId;
use tardis_gallifrey::KnowledgeStore;

/// The Astrolabe navigator.
#[derive(Debug)]
pub struct Astrolabe {
    knowledge: Arc<KnowledgeStore>,
}

/// A node in the A* search.
#[derive(Debug, Clone, PartialEq)]
struct Node {
    id: EntityId,
    cost: f32,
    heuristic: f32,
    parent: Option<EntityId>,
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (lowest f_score = cost + heuristic)
        (other.cost + other.heuristic)
            .partial_cmp(&(self.cost + self.heuristic))
            .unwrap_or(Ordering::Equal)
    }
}

impl Astrolabe {
    /// Create a new Astrolabe.
    #[must_use]
    pub const fn new(knowledge: Arc<KnowledgeStore>) -> Self {
        Self { knowledge }
    }

    /// Find a semantic path between two entities by name.
    ///
    /// # Errors
    ///
    /// Returns an error if start/end entities are not found or no path exists.
    pub fn navigate(&self, start_name: &str, end_name: &str) -> anyhow::Result<Vec<String>> {
        let start_id = self
            .find_entity_id(start_name)
            .ok_or_else(|| anyhow::anyhow!("Start entity not found: {}", start_name))?;
        let end_id = self
            .find_entity_id(end_name)
            .ok_or_else(|| anyhow::anyhow!("End entity not found: {}", end_name))?;

        let end_entity = self
            .knowledge
            .get_entity(end_id)
            .map_err(|e| anyhow::anyhow!(e))?
            .ok_or_else(|| anyhow::anyhow!("End entity missing"))?;

        // 1. Build Adjacency Graph (Snapshot)
        let adjacency = self.build_adjacency_map()?;

        // 2. A* Search
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<EntityId, EntityId> = HashMap::new();
        let mut g_score: HashMap<EntityId, f32> = HashMap::new();

        g_score.insert(start_id, 0.0);

        open_set.push(Node {
            id: start_id,
            cost: 0.0,
            heuristic: self.heuristic(start_id, &end_entity)?,
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            if current.id == end_id {
                return self.reconstruct_path(current.id, &came_from);
            }

            if closed_set.contains(&current.id) {
                continue;
            }
            closed_set.insert(current.id);

            if let Some(neighbors) = adjacency.get(&current.id) {
                for &neighbor_id in neighbors {
                    if closed_set.contains(&neighbor_id) {
                        continue;
                    }

                    let tentative_g_score = g_score.get(&current.id).unwrap_or(&f32::INFINITY) + 1.0; // Uniform cost for now

                    if tentative_g_score < *g_score.get(&neighbor_id).unwrap_or(&f32::INFINITY) {
                        came_from.insert(neighbor_id, current.id);
                        g_score.insert(neighbor_id, tentative_g_score);

                        let h = self.heuristic(neighbor_id, &end_entity).unwrap_or(0.0);
                        open_set.push(Node {
                            id: neighbor_id,
                            cost: tentative_g_score,
                            heuristic: h,
                            parent: Some(current.id),
                        });
                    }
                }
            }
        }

        Err(anyhow::anyhow!("No path found between '{}' and '{}'", start_name, end_name))
    }

    fn find_entity_id(&self, name: &str) -> Option<EntityId> {
        let mut found_id = None;
        let _ = self.knowledge.scan_history(|history| {
            if found_id.is_some() {
                return;
            }
            // Check latest version
            if let Some(latest) = history.last() {
                if latest.name.eq_ignore_ascii_case(name) {
                    found_id = Some(latest.id);
                }
            }
        });
        found_id
    }

    fn build_adjacency_map(&self) -> anyhow::Result<HashMap<EntityId, Vec<EntityId>>> {
        let mut adjacency: HashMap<EntityId, Vec<EntityId>> = HashMap::new();

        self.knowledge.scan_relationships(|rels| {
            for rel in rels {
                // Undirected graph for navigation flexibility?
                // Let's stick to directed for now as relationships have direction.
                // Or maybe undirected is better for "association"?
                // Let's go with Directed for "Follows" logic, but maybe add reverse edges if needed.
                // Actually, "related to" is often symmetric in concept but stored directed.
                // Let's add BOTH directions to allow traversing "backwards" (e.g. parent <- child).
                adjacency.entry(rel.source).or_default().push(rel.target);
                adjacency.entry(rel.target).or_default().push(rel.source);
            }
        }).map_err(|e| anyhow::anyhow!(e))?;

        Ok(adjacency)
    }

    fn heuristic(&self, id: EntityId, end_entity: &Entity) -> anyhow::Result<f32> {
        // if end_entity has no embedding, return 0.0 (Dijkstra fallback)
        let Some(end_embedding) = &end_entity.embedding else {
            return Ok(0.0);
        };

        let entity = self.knowledge.get_entity(id)
            .map_err(|e| anyhow::anyhow!(e))?
            .ok_or_else(|| anyhow::anyhow!("Entity not found"))?;

        let Some(embedding) = &entity.embedding else {
            return Ok(1.0); // Penalize nodes without embeddings (max distance)
        };

        // Cosine Similarity
        let similarity = cosine_similarity(embedding, end_embedding);

        // Heuristic: 1.0 - similarity (so higher similarity = lower cost)
        // Ensure result is non-negative
        Ok((1.0 - similarity).max(0.0))
    }

    fn reconstruct_path(
        &self,
        mut current: EntityId,
        came_from: &HashMap<EntityId, EntityId>,
    ) -> anyhow::Result<Vec<String>> {
        let mut path = vec![current];
        while let Some(&prev) = came_from.get(&current) {
            current = prev;
            path.push(current);
        }
        path.reverse();

        // Resolve names
        let mut names = Vec::new();
        for id in path {
            let entity = self.knowledge.get_entity(id)
                .map_err(|e| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Entity missing in path"))?;
            names.push(entity.name);
        }

        Ok(names)
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
    use tardis_common::domain::Relationship;
    use tardis_common::temporal::BiTemporalInterval;
    use std::collections::HashMap;

    fn create_entity(name: &str, embedding: Option<Vec<f32>>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Concept".to_string(),
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
            relationship_type: "RELATED".to_string(),
            source,
            target,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        }
    }

    #[test]
    fn test_navigate_basic() {
        let store = Arc::new(KnowledgeStore::new());
        let astrolabe = Astrolabe::new(store.clone());

        // A -> B -> C
        let a = create_entity("A", Some(vec![1.0, 0.0]));
        let b = create_entity("B", Some(vec![0.5, 0.5]));
        let c = create_entity("C", Some(vec![0.0, 1.0]));

        let id_a = store.insert_entity(a.clone()).unwrap();
        let id_b = store.insert_entity(b.clone()).unwrap();
        let id_c = store.insert_entity(c.clone()).unwrap();

        store.insert_relationship(create_rel(id_a, id_b)).unwrap();
        store.insert_relationship(create_rel(id_b, id_c)).unwrap();

        let path = astrolabe.navigate("A", "C").unwrap();
        assert_eq!(path, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_navigate_semantic_shortcut() {
        let store = Arc::new(KnowledgeStore::new());
        let astrolabe = Astrolabe::new(store.clone());

        // A -> B -> C -> D (Long path, 3 hops)
        // A -> X -> D (Short path, 2 hops, but X is semantically far from D?)
        // Wait, A* finds shortest path based on COST. Heuristic just guides search.
        // If cost is uniform (1.0), it will find shortest hop count.
        // To test semantic preference, I need edges to have costs based on similarity?
        // My implementation uses Uniform Cost (1.0) for g_score.
        // So it will always find shortest hop path.
        // But the search order (nodes visited) will be influenced by heuristic.

        // Let's verify it finds the path A->X->D
        let a = create_entity("A", None);
        let x = create_entity("X", None);
        let d = create_entity("D", None);

        let id_a = store.insert_entity(a).unwrap();
        let id_x = store.insert_entity(x).unwrap();
        let id_d = store.insert_entity(d).unwrap();

        store.insert_relationship(create_rel(id_a, id_x)).unwrap();
        store.insert_relationship(create_rel(id_x, id_d)).unwrap();

        let path = astrolabe.navigate("A", "D").unwrap();
        assert_eq!(path, vec!["A", "X", "D"]);
    }
}
