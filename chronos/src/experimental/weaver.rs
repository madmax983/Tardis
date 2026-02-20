//! The Weaver: Narrative generator for entity connections.
//!
//! "Everything is connected."
//!
//! This module finds paths between entities in the knowledge graph and weaves a narrative
//! using the LLM to explain the connection.

use crate::error::ChronosResult;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::config::InferenceParams;
use tardis_vortex::model::ModelHandle;
use tardis_vortex::Vortex;

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model_handle: ModelHandle,
}

impl Weaver {
    /// Create a new Weaver instance.
    #[must_use]
    pub const fn new(
        gallifrey: Arc<Gallifrey>,
        vortex: Arc<Vortex>,
        model_handle: ModelHandle,
    ) -> Self {
        Self {
            gallifrey,
            vortex,
            model_handle,
        }
    }

    /// Weave a narrative connecting two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities not found or path cannot be generated.
    pub async fn weave(&self, start_name: &str, end_name: &str) -> ChronosResult<String> {
        // 1. Resolve names to IDs (Naive scan)
        let start_id = self.find_entity_id(start_name)?;
        let end_id = self.find_entity_id(end_name)?;

        // 2. Build Adjacency Graph (Naive scan of relationships)
        let graph = self.build_graph()?;

        // 3. Find Path (BFS)
        let path = Self::find_path(start_id, end_id, &graph);

        if let Some(path_ids) = path {
            // 4. Collect Path Details
            let entities = self.get_entities(&path_ids)?;

            // 5. Generate Narrative
            let narrative = self.generate_narrative(&entities).await?;
            Ok(narrative)
        } else {
            Ok(format!(
                "The threads of time do not connect '{start_name}' and '{end_name}'."
            ))
        }
    }

    fn find_entity_id(&self, name: &str) -> ChronosResult<EntityId> {
        let mut found_id = None;
        let target_name = name.to_lowercase();

        // Use scan_history to iterate all entities
        self.gallifrey.knowledge().scan_history(|history| {
            if found_id.is_some() {
                return;
            }
            // Check the most recent version
            if let Some(entity) = history.last() {
                if entity.name.to_lowercase() == target_name {
                    found_id = Some(entity.id);
                }
            }
        })?;

        found_id.ok_or_else(|| {
            crate::error::ChronosError::Common(tardis_common::Error::EntityNotFound(format!(
                "Entity '{name}' not found"
            )))
        })
    }

    fn build_graph(&self) -> ChronosResult<HashMap<EntityId, Vec<EntityId>>> {
        let mut adjacency: HashMap<EntityId, Vec<EntityId>> = HashMap::new();

        self.gallifrey.knowledge().scan_relationships(|rels| {
            for rel in rels {
                // Undirected graph for narrative purposes
                adjacency.entry(rel.source).or_default().push(rel.target);
                adjacency.entry(rel.target).or_default().push(rel.source);
            }
        })?;

        Ok(adjacency)
    }

    fn find_path(
        start: EntityId,
        end: EntityId,
        graph: &HashMap<EntityId, Vec<EntityId>>,
    ) -> Option<Vec<EntityId>> {
        let mut queue = VecDeque::new();
        queue.push_back(vec![start]);
        let mut visited = HashSet::new();
        visited.insert(start);

        while let Some(path) = queue.pop_front() {
            let Some(&current) = path.last() else {
                continue;
            };

            if current == end {
                return Some(path);
            }

            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor);
                        queue.push_back(new_path);
                    }
                }
            }
        }
        None
    }

    fn get_entities(&self, ids: &[EntityId]) -> ChronosResult<Vec<Entity>> {
        let mut entities = Vec::new();
        for &id in ids {
            if let Some(entity) = self.gallifrey.knowledge().get_entity(id)? {
                entities.push(entity);
            }
        }
        Ok(entities)
    }

    async fn generate_narrative(&self, entities: &[Entity]) -> ChronosResult<String> {
        if entities.is_empty() {
            return Ok("No entities in path.".to_string());
        }

        use std::fmt::Write;
        let mut prompt = String::from("Based on the following sequence of connected entities, write a short, cohesive narrative explaining their relationship:\n\n");

        for (i, entity) in entities.iter().enumerate() {
            let _ = write!(
                prompt,
                "{}. Name: {}\nType: {}\n",
                i + 1,
                entity.name,
                entity.entity_type
            );
            if let Some(desc) = entity
                .properties
                .get("description")
                .and_then(|v| v.as_str())
            {
                let _ = writeln!(prompt, "Description: {desc}");
            }
            prompt.push('\n');
        }

        prompt.push_str("Narrative:");

        let params = InferenceParams {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            ..InferenceParams::default()
        };

        Ok(self
            .vortex
            .infer(self.model_handle, &prompt, params)
            .await?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::{Entity, Relationship};
    use tardis_gallifrey::Gallifrey;
    use tardis_vortex::model::ModelHandle;
    use tardis_vortex::Vortex;

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

    #[tokio::test]
    async fn test_weaver_path() {
        // Setup Gallifrey
        let gallifrey = Arc::new(Gallifrey::new());
        let e1 = create_entity("StartNode");
        let e2 = create_entity("MiddleNode");
        let e3 = create_entity("EndNode");

        gallifrey.insert(e1.clone()).await.unwrap();
        gallifrey.insert(e2.clone()).await.unwrap();
        gallifrey.insert(e3.clone()).await.unwrap();

        // A -> B -> C
        let rel1 = create_rel(e1.id, e2.id);
        let rel2 = create_rel(e2.id, e3.id);

        gallifrey.knowledge().insert_relationship(rel1).unwrap();
        gallifrey.knowledge().insert_relationship(rel2).unwrap();

        // Setup Vortex
        let vortex = Arc::new(Vortex::new().unwrap());
        let handle = ModelHandle::new(1);

        // Mock inference
        vortex.set_mock_inference(Box::new(|_, _prompt, _| {
            Ok("A story about connection.".to_string())
        }));

        let weaver = Weaver::new(gallifrey, vortex, handle);

        let result = weaver.weave("StartNode", "EndNode").await.unwrap();
        assert_eq!(result, "A story about connection.");
    }

    #[tokio::test]
    async fn test_weaver_no_path() {
        // Setup Gallifrey
        let gallifrey = Arc::new(Gallifrey::new());
        let e1 = create_entity("LonelyNode");
        let e2 = create_entity("FarAwayNode");

        gallifrey.insert(e1.clone()).await.unwrap();
        gallifrey.insert(e2.clone()).await.unwrap();

        // No relationships

        let vortex = Arc::new(Vortex::new().unwrap());
        let handle = ModelHandle::new(1);

        let weaver = Weaver::new(gallifrey, vortex, handle);

        let result = weaver.weave("LonelyNode", "FarAwayNode").await.unwrap();
        assert!(result.contains("do not connect"));
    }
}
