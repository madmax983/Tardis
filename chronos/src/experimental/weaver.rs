//! The Weaver 🕸️
//!
//! A narrative engine that connects seemingly unrelated entities through creative storytelling.
//!
//! The Weaver takes two entities from the Knowledge Graph and uses the LLM to generate
//! a plausible (or fanciful) causal chain or thematic connection between them.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl Weaver {
    /// Create a new Weaver instance.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>, model: ModelHandle) -> Self {
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
    /// Returns an error if entities are not found or inference fails.
    pub async fn weave(&self, entity1_name: &str, entity2_name: &str) -> Result<String> {
        let e1 = self.find_entity(entity1_name)?;
        let e2 = self.find_entity(entity2_name)?;

        let prompt = format!(
            "<s>[INST] You are The Weaver, a narrative engine for the Tardis OS.
Your task is to create a creative, plausible, and interesting narrative connection between two system entities.
Use the provided metadata to spin a short story (max 3 sentences) explaining how they are related.
It can be technical, metaphorical, or sci-fi/time-travel themed.

Entity 1: {} ({})
Properties: {:?}

Entity 2: {} ({})
Properties: {:?}

The connection is: [/INST]",
            e1.name, e1.entity_type, e1.properties, e2.name, e2.entity_type, e2.properties
        );

        let params = InferenceParams::default()
            .with_temperature(0.8)
            .with_max_tokens(150);

        let result = self
            .vortex
            .infer(self.model, &prompt, params)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(result)
    }

    fn find_entity(&self, name: &str) -> Result<Entity> {
        let knowledge = self.gallifrey.knowledge();
        let mut found = None;

        // Scan all entities to find the one with the matching name (case-insensitive)
        knowledge.scan_history(|history| {
            if found.is_some() {
                return;
            }

            // Find latest version that matches name
            if let Some(e) = history.iter().find(|e| {
                e.name.eq_ignore_ascii_case(name) && e.temporal.is_current()
            }) {
                found = Some(e.clone());
            }
        })?;

        found.ok_or_else(|| anyhow!("Entity '{name}' not found in current time"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str, entity_type: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: Some("test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_find_entity_manual() {
        let gallifrey = Arc::new(Gallifrey::new());
        // We can't easily mock Vortex here without loading a model, so we'll just test the entity lookup part.
        // To test the full Weaver we'd need a mock Vortex or a way to bypass inference.
        // For now, let's just verifying finding entities works.

        let entity = create_test_entity("TestBot", "Bot");
        gallifrey.insert(entity).await.unwrap();

        let knowledge = gallifrey.knowledge();
        let mut found = None;
        knowledge.scan_history(|history| {
             if let Some(e) = history.iter().find(|e| e.name == "TestBot") {
                 found = Some(e.clone());
             }
        }).unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "TestBot");
    }
}
