//! The Weaver: Narrative Connection Engine.
//!
//! "We are all stories in the end."
//!
//! This module uses `Vortex` to generate narrative connections between two entities
//! in the `Gallifrey` knowledge graph, "weaving" them together into a coherent story.
//!
//! If a connection is successfully generated, it is stored as a `Relationship`
//! of type "`NARRATIVE_LINK`".

use crate::error::ChronosResult;
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_common::temporal::BiTemporalInterval;
use tardis_gallifrey::domain::{Entity, Relationship};
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};
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

    /// Weave a narrative connection between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or inference fails.
    #[instrument(skip(self))]
    pub async fn weave(&self, name_a: &str, name_b: &str) -> ChronosResult<String> {
        info!("Weaving narrative between '{name_a}' and '{name_b}'");

        let entity_a = self.find_entity(name_a)?;
        let entity_b = self.find_entity(name_b)?;

        // Construct prompt
        let mut prompt = String::new();
        let _ = writeln!(&mut prompt, "You are a creative historian. Your task is to find a plausible or creative connection between two entities in the Tardis OS universe.");
        let _ = writeln!(&mut prompt, "\nEntity A:");
        let _ = writeln!(&mut prompt, "Name: {}", entity_a.name);
        let _ = writeln!(&mut prompt, "Type: {}", entity_a.entity_type);
        let _ = writeln!(&mut prompt, "Properties: {:?}", entity_a.properties);

        let _ = writeln!(&mut prompt, "\nEntity B:");
        let _ = writeln!(&mut prompt, "Name: {}", entity_b.name);
        let _ = writeln!(&mut prompt, "Type: {}", entity_b.entity_type);
        let _ = writeln!(&mut prompt, "Properties: {:?}", entity_b.properties);

        let _ = writeln!(&mut prompt, "\nTask: Write a short paragraph explaining how these two entities might be connected. If no obvious connection exists, invent a scenario where they interact.");
        let _ = writeln!(&mut prompt, "Narrative Connection:");

        // Ask Vortex
        let params = InferenceParams::default()
            .with_temperature(0.8) // High creativity
            .with_max_tokens(256);

        let story = self.vortex.infer(self.model, &prompt, params).await?;

        // Store the connection in Gallifrey
        self.store_connection(&entity_a, &entity_b, &story)?;

        Ok(story)
    }

    fn find_entity(&self, name: &str) -> ChronosResult<Entity> {
        let mut found = None;

        // Scan history to find the entity
        // We use the latest version of the first match
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                if found.is_some() {
                    return;
                }
                if let Some(latest) = history.last() {
                    if latest.name.eq_ignore_ascii_case(name) {
                        found = Some(latest.clone());
                    }
                }
            })
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        found.ok_or_else(|| {
            crate::error::ChronosError::Common(tardis_common::Error::EntityNotFound(format!(
                "Entity '{name}' not found"
            )))
        })
    }

    fn store_connection(
        &self,
        source: &Entity,
        target: &Entity,
        story: &str,
    ) -> ChronosResult<()> {
        let relationship = Relationship {
            id: EntityId::new(),
            relationship_type: "NARRATIVE_LINK".to_string(),
            source: source.id,
            target: target.id,
            properties: std::collections::HashMap::from([(
                "narrative".to_string(),
                serde_json::Value::String(story.to_string()),
            )]),
            temporal: BiTemporalInterval::now(),
        };

        self.gallifrey
            .knowledge()
            .insert_relationship(relationship)
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
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
    async fn test_weaver_connects_entities() {
        // Setup
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Insert entities
        let e1 = create_entity("Doctor");
        let e2 = create_entity("Dalek");
        gallifrey.insert(e1.clone()).await.unwrap();
        gallifrey.insert(e2.clone()).await.unwrap();

        // Mock Vortex
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("The Doctor and the Dalek are eternal enemies.".to_string())
        }));

        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let weaver = Weaver::new(gallifrey.clone(), vortex, handle);

        // Test
        let story = weaver.weave("Doctor", "Dalek").await.unwrap();

        assert!(story.contains("eternal enemies"));

        // Verify relationship created
        let mut found_rel = false;
        gallifrey
            .knowledge()
            .scan_relationships(|rels| {
                for rel in rels {
                    if rel.source == e1.id && rel.target == e2.id && rel.relationship_type == "NARRATIVE_LINK" {
                        found_rel = true;
                    }
                }
            })
            .unwrap();

        assert!(found_rel, "Relationship should be created");
    }

    #[tokio::test]
    async fn test_weaver_entity_not_found() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let weaver = Weaver::new(gallifrey, vortex, handle);

        let result = weaver.weave("Ghost", "Shadow").await;
        assert!(result.is_err());
    }
}
