//! The Weaver 🕸️
//!
//! A narrative engine that connects seemingly unrelated entities through creative storytelling.
//!
//! The Weaver takes two entities from the Knowledge Graph and uses the LLM to generate
//! a plausible (or fanciful) causal chain or thematic connection between them.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tardis_common::domain::Entity;
use tardis_common::id::ModelHandle;
use tardis_common::llm::InferenceParams;
use tardis_common::traits::{KnowledgeService, LlmService};

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    knowledge: Arc<dyn KnowledgeService>,
    llm: Arc<dyn LlmService>,
    model: Option<ModelHandle>,
}

impl Weaver {
    /// Create a new Weaver instance.
    #[must_use]
    pub fn new(
        knowledge: Arc<dyn KnowledgeService>,
        llm: Arc<dyn LlmService>,
        model: Option<ModelHandle>,
    ) -> Self {
        Self {
            knowledge,
            llm,
            model,
        }
    }

    /// Weave a narrative connecting two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or inference fails.
    pub async fn weave(&self, entity1_name: &str, entity2_name: &str) -> Result<String> {
        let e1 = self.find_entity(entity1_name).await?;
        let e2 = self.find_entity(entity2_name).await?;

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

        let mut params = InferenceParams::default()
            .with_temperature(0.8)
            .with_max_tokens(150);

        if let Some(model) = self.model {
            params = params.with_model(model);
        }

        let result = self
            .llm
            .infer(&prompt, params)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(result)
    }

    async fn find_entity(&self, name: &str) -> Result<Entity> {
        self.knowledge
            .find_entity_by_name(name)
            .await
            .map_err(|e| anyhow!(e.to_string()))?
            .ok_or_else(|| anyhow!("Entity '{name}' not found in current time"))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::Gallifrey;

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
        let knowledge: Arc<dyn KnowledgeService> = gallifrey.knowledge();
        // We can't easily mock Vortex here without loading a model, so we'll just test the entity lookup part.
        // To test the full Weaver we'd need a mock Vortex or a way to bypass inference.
        // For now, let's just verifying finding entities works via the trait.

        let entity = create_test_entity("TestBot", "Bot");
        knowledge.insert_entity(entity).await.unwrap();

        let found = knowledge.find_entity_by_name("TestBot").await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "TestBot");
    }
}
