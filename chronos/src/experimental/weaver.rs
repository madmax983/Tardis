//! # The Weaver
//!
//! A module that finds and narrates hidden connections between entities in the knowledge graph.

use crate::ChronosResult;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// A step in a connection path.
#[derive(Debug, Clone)]
pub struct ConnectionStep {
    /// The entity we came from.
    pub from: EntityId,
    /// The entity we went to.
    pub to: EntityId,
    /// The relationship type.
    pub rel_type: String,
}

/// The Weaver service.
#[derive(Debug)]
pub struct Weaver {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
}

impl Weaver {
    /// Create a new Weaver.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>) -> Self {
        Self { gallifrey, vortex }
    }

    /// Find a path between two entities.
    ///
    /// Uses BFS to find the shortest path.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge graph cannot be accessed.
    ///
    /// # Panics
    ///
    /// Panics if the path reconstruction logic fails (which should be impossible).
    #[instrument(skip(self))]
    #[allow(clippy::expect_used)]
    pub fn find_connection(
        &self,
        start: EntityId,
        end: EntityId,
        at: DateTime<Utc>,
    ) -> ChronosResult<Option<Vec<ConnectionStep>>> {
        info!("Weaving connection from {} to {}", start, end);

        // 1. Build Adjacency List (in-memory, filtered by time)
        // Map: Source -> Vec<(Target, Type)>
        let mut adjacency: HashMap<EntityId, Vec<(EntityId, String)>> = HashMap::new();

        self.gallifrey
            .knowledge()
            .scan_relationships(|history| {
                // Find version active at `at`
                if let Some(rel) = history.iter().find(|r| r.temporal.active_at(at, at)) {
                    adjacency
                        .entry(rel.source)
                        .or_default()
                        .push((rel.target, rel.relationship_type.clone()));
                }
            })
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        // 2. BFS
        let mut queue = VecDeque::new();
        queue.push_back(start);

        let mut visited = HashSet::new();
        visited.insert(start);

        // Track path: Child -> (Parent, RelType)
        let mut parents: HashMap<EntityId, (EntityId, String)> = HashMap::new();

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut curr = end;
                while curr != start {
                    let (parent, rel_type) = parents
                        .get(&curr)
                        .expect("Parent must exist for visited node");
                    path.push(ConnectionStep {
                        from: *parent,
                        to: curr,
                        rel_type: rel_type.clone(),
                    });
                    curr = *parent;
                }
                path.reverse();
                return Ok(Some(path));
            }

            if let Some(neighbors) = adjacency.get(&current) {
                for (neighbor, rel_type) in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(*neighbor);
                        parents.insert(*neighbor, (current, rel_type.clone()));
                        queue.push_back(*neighbor);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Narrate the connection between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if pathfinding or inference fails.
    ///
    /// # Panics
    ///
    /// Panics if entity names are missing from the map (which should be impossible).
    #[allow(clippy::expect_used)]
    pub async fn narrate_connection(
        &self,
        start: EntityId,
        end: EntityId,
    ) -> ChronosResult<String> {
        let path = self.find_connection(start, end, Utc::now())?;

        let Some(path) = path else {
            return Ok(format!("No connection found between {start} and {end}."));
        };

        // Fetch entity names for better narrative
        let mut names = HashMap::new();
        let mut ids_to_fetch = HashSet::new();
        ids_to_fetch.insert(start);
        ids_to_fetch.insert(end);
        for step in &path {
            ids_to_fetch.insert(step.from);
            ids_to_fetch.insert(step.to);
        }

        for id in ids_to_fetch {
            if let Ok(Some(entity)) = self.gallifrey.knowledge().get_entity(id) {
                names.insert(id, entity.name);
            } else {
                names.insert(id, id.to_string());
            }
        }

        let start_name = names.get(&start).expect("Start name missing");
        let end_name = names.get(&end).expect("End name missing");

        let mut path_desc = String::new();
        for step in path {
            let from_name = names.get(&step.from).expect("From name missing");
            let to_name = names.get(&step.to).expect("To name missing");
            writeln!(
                &mut path_desc,
                "{from_name} --[{}]--> {to_name}",
                step.rel_type
            )
            .ok();
        }

        let prompt = format!(
            "You are The Weaver. Explain the hidden connection between '{start_name}' and '{end_name}' based on this path:\n\n{path_desc}\n\
            Write a short, intriguing summary of how they are connected."
        );

        let handle = self
            .vortex
            .load_model("default", tardis_common::llm::ModelLoadConfig::default())
            .await?;

        let narrative = self
            .vortex
            .infer(
                handle,
                &prompt,
                tardis_common::llm::InferenceParams::default(),
            )
            .await?;

        Ok(narrative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tardis_common::domain::{Entity, Relationship};
    use tardis_common::id::ModelHandle;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_entity(id: EntityId, name: &str) -> Entity {
        Entity {
            id,
            entity_type: "Person".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    fn create_rel(source: EntityId, target: EntityId, rel_type: &str) -> Relationship {
        Relationship {
            id: EntityId::new(),
            relationship_type: rel_type.to_string(),
            source,
            target,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        }
    }

    #[tokio::test]
    async fn test_weaver_find_connection() {
        let gallifrey = Arc::new(Gallifrey::new());
        // Use default mock which is empty, but we won't call load/infer so it's fine
        let vortex = Arc::new(Vortex::new().unwrap());

        // Setup graph: A -> B -> C
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();

        gallifrey
            .knowledge()
            .insert_entity(create_entity(a, "Alice"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_entity(create_entity(b, "Bob"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_entity(create_entity(c, "Charlie"))
            .unwrap();

        gallifrey
            .knowledge()
            .insert_relationship(create_rel(a, b, "KNOWS"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_relationship(create_rel(b, c, "HIRES"))
            .unwrap();

        let weaver = Weaver::new(gallifrey, vortex);
        let path = weaver.find_connection(a, c, Utc::now()).unwrap().unwrap();

        assert_eq!(path.len(), 2);
        assert_eq!(path[0].from, a);
        assert_eq!(path[0].to, b);
        assert_eq!(path[0].rel_type, "KNOWS");
        assert_eq!(path[1].from, b);
        assert_eq!(path[1].to, c);
        assert_eq!(path[1].rel_type, "HIRES");
    }

    #[tokio::test]
    async fn test_weaver_narrative() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Vortex
        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(1))));
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("Alice") && prompt.contains("Charlie") {
                return Ok("Alice knows Bob who hired Charlie.".to_string());
            }
            Ok("Unknown".to_string())
        }));

        // Setup graph
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();

        gallifrey
            .knowledge()
            .insert_entity(create_entity(a, "Alice"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_entity(create_entity(b, "Bob"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_entity(create_entity(c, "Charlie"))
            .unwrap();

        gallifrey
            .knowledge()
            .insert_relationship(create_rel(a, b, "KNOWS"))
            .unwrap();
        gallifrey
            .knowledge()
            .insert_relationship(create_rel(b, c, "HIRES"))
            .unwrap();

        let weaver = Weaver::new(gallifrey, vortex);
        let story = weaver.narrate_connection(a, c).await.unwrap();

        assert!(story.contains("Alice knows Bob"));
    }
}
