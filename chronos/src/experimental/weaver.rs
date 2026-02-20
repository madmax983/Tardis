//! The Weaver: Narrative Path Generator.
//!
//! "We are all connected. Sometimes the thread is just hard to see."
//!
//! This module finds connections between disparate entities in the Knowledge Graph
//! and uses Vortex (LLM) to weave a narrative explanation of their relationship.

use crate::error::{ChronosError, ChronosResult};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::config::InferenceParams;
use tardis_vortex::{ModelHandle, Vortex};
use tracing::{info, instrument};

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl Weaver {
    /// Create a new Weaver engine.
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

    /// Weave a narrative connecting two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if the entities are not found, no path exists, or inference fails.
    #[instrument(skip(self))]
    pub async fn weave(&self, start_name: &str, end_name: &str) -> ChronosResult<String> {
        info!("Weaving path from '{}' to '{}'", start_name, end_name);

        let knowledge = self.gallifrey.knowledge();

        // 1. Find Start and End IDs
        let mut start_id = None;
        let mut end_id = None;

        // Linear scan for now (experimental feature)
        knowledge
            .scan_history(|history| {
                // Check mostly recent version
                if let Some(entity) = history.last() {
                    // We check name regardless of casing, but exact match is preferred
                    if entity.temporal.is_current() {
                        if entity.name.eq_ignore_ascii_case(start_name) {
                            start_id = Some(entity.id);
                        } else if entity.name.eq_ignore_ascii_case(end_name) {
                            end_id = Some(entity.id);
                        }
                    }
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let start_id = start_id.ok_or_else(|| {
            ChronosError::Common(tardis_common::Error::Internal(format!(
                "Entity not found: {start_name}"
            )))
        })?;
        let end_id = end_id.ok_or_else(|| {
            ChronosError::Common(tardis_common::Error::Internal(format!(
                "Entity not found: {end_name}"
            )))
        })?;

        if start_id == end_id {
            return Ok(format!(
                "'{start_name}' is '{end_name}'. They are one and the same."
            ));
        }

        // 2. Build Adjacency Graph (In-Memory)
        let mut adjacency: HashMap<EntityId, Vec<EntityId>> = HashMap::new();

        knowledge
            .scan_relationships(|rels| {
                for rel in rels {
                    if rel.temporal.is_current() {
                        adjacency.entry(rel.source).or_default().push(rel.target);
                        // Also add reverse edge for "undirected" traversal if desired?
                        // "Weaver" implies finding *any* connection. Often relationships like "OwnedBy" imply "Owns" in reverse.
                        // Let's add reverse edges to make it easier to find connections.
                        adjacency.entry(rel.target).or_default().push(rel.source);
                    }
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // 3. BFS for Shortest Path
        let mut queue = VecDeque::new();
        queue.push_back(vec![start_id]);
        let mut visited = HashSet::new();
        visited.insert(start_id);

        let mut path_found: Option<Vec<EntityId>> = None;

        while let Some(path) = queue.pop_front() {
            let current = *path.last().unwrap();
            if current == end_id {
                path_found = Some(path);
                break;
            }

            // Limit depth to avoid infinite loops or massive search
            if path.len() > 6 {
                continue;
            }

            if let Some(neighbors) = adjacency.get(&current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor);
                        queue.push_back(new_path);
                    }
                }
            }
        }

        let path_ids = path_found.ok_or_else(|| {
            ChronosError::Common(tardis_common::Error::Internal(format!(
                "No connection found between '{start_name}' and '{end_name}' within 6 hops."
            )))
        })?;

        // 4. Retrieve Entity Details for Prompt
        let mut path_descriptions = Vec::new();
        for id in &path_ids {
            if let Some(entity) = knowledge
                .get_entity(*id)
                .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?
            {
                let desc = entity
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                path_descriptions.push(format!("- {}: {} ({})", entity.name, entity.entity_type, desc));
            }
        }

        // 5. Generate Narrative
        let prompt = format!(
            "I have found a path connecting these two concepts:\n\
             Start: {}\n\
             End: {}\n\n\
             The path consists of the following steps:\n\
             {}\n\n\
             Write a short, engaging paragraph explaining the connection between '{}' and '{}' based on this path. Be creative but accurate to the provided steps.",
            start_name,
            end_name,
            path_descriptions.join("\n"),
            start_name,
            end_name
        );

        let params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(256);

        let response = self.vortex.infer(self.model, &prompt, params).await?;

        Ok(response)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::{Entity, Relationship};
    use tardis_vortex::ModelLoadConfig;

    fn create_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_weaver_traversal() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Setup Graph: A -> B -> C
        let a = create_entity("Alice");
        let b = create_entity("Bob");
        let c = create_entity("Charlie");

        let id_a = a.id;
        let id_b = b.id;
        let id_c = c.id;

        gallifrey.insert(a).await.unwrap();
        gallifrey.insert(b).await.unwrap();
        gallifrey.insert(c).await.unwrap();

        // Relationships
        let rel1 = Relationship {
            id: EntityId::new(),
            relationship_type: "KNOWS".to_string(),
            source: id_a,
            target: id_b,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };
        let rel2 = Relationship {
            id: EntityId::new(),
            relationship_type: "KNOWS".to_string(),
            source: id_b,
            target: id_c,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };

        gallifrey
            .knowledge()
            .insert_relationship(rel1)
            .unwrap();
        gallifrey
            .knowledge()
            .insert_relationship(rel2)
            .unwrap();

        // Mock Vortex
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("Alice knows Bob who knows Charlie.".to_string())
        }));

        // Mock Load Model
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let weaver = Weaver::new(gallifrey, vortex, handle);

        // Test
        let narrative = weaver.weave("Alice", "Charlie").await.unwrap();
        assert_eq!(narrative, "Alice knows Bob who knows Charlie.");
    }
}
