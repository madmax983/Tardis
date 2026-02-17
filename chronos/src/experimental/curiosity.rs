//! The Curiosity Engine: Active Learning for Tardis.
//!
//! "Curiosity killed the cat, but satisfaction brought it back."
//!
//! This module scans the knowledge graph for sparse or ambiguous entities
//! and generates questions to ask the user, turning the OS into an active learner.

use crate::error::ChronosResult;
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::config::InferenceParams;
use tardis_vortex::{ModelHandle, Vortex};
use tracing::{info, instrument};

/// The Curiosity engine.
#[derive(Debug)]
pub struct Curiosity {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl Curiosity {
    /// Create a new Curiosity engine.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>, model: ModelHandle) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Scan for an ambiguous entity and formulate a question about it.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails or inference fails.
    #[instrument(skip(self))]
    pub async fn ask(&self) -> ChronosResult<String> {
        info!("Curiosity: Scanning for knowledge gaps...");

        let mut target_entity: Option<Entity> = None;

        // Scan history to find a sparse entity
        // Heuristic: Has fewer than 3 properties and is not a "System" type
        // We stop at the first one we find for now (MVP).
        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                if target_entity.is_some() {
                    return;
                }

                // Look at the latest version of the entity
                if let Some(latest) = history.last() {
                    if latest.properties.len() < 3
                        && latest.entity_type != "System"
                        && !latest.name.is_empty()
                    {
                        target_entity = Some(latest.clone());
                    }
                }
            })
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        let Some(entity) = target_entity else {
            return Ok("I am content. My knowledge base feels complete... for now.".to_string());
        };

        info!(
            "Curiosity: Found gap in entity '{}' ({})",
            entity.name, entity.entity_type
        );

        // Construct prompt
        let prompt = format!(
            "I have an entity in my database that is sparse. I want to learn more about it.\n\
             Entity: {{ \"name\": \"{}\", \"type\": \"{}\", \"properties\": {:?} }}\n\
             Task: Write a single, short, friendly question to the user asking for more details about this entity's purpose or origin.\n\
             Question:",
            entity.name, entity.entity_type, entity.properties
        );

        // Ask Vortex
        let params = InferenceParams::default()
            .with_temperature(0.8) // Be creative
            .with_max_tokens(64);

        let question = self.vortex.infer(self.model, &prompt, params).await?;

        Ok(format!(
            "🤔 Regarding '{}': {}",
            entity.name,
            question.trim()
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_vortex::ModelLoadConfig;

    fn create_sparse_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Concept".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_curiosity_asks_question() {
        // Setup dependencies
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Insert sparse entity
        let entity = create_sparse_entity("Mystery Box");
        gallifrey.insert(entity).await.unwrap();

        // Mock Vortex
        let mock_handle = ModelHandle::new(1);
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(mock_handle)));

        vortex.set_mock_inference(Box::new(
            |_, _, _| Ok("What is inside the box?".to_string()),
        ));

        // Load dummy model to register handle
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let curiosity = Curiosity::new(gallifrey, vortex, handle);

        let question = curiosity.ask().await.unwrap();

        assert!(question.contains("Mystery Box"));
        assert!(question.contains("What is inside the box?"));
    }

    #[tokio::test]
    async fn test_curiosity_no_gaps() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // No entities

        // Register dummy model
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let curiosity = Curiosity::new(gallifrey, vortex, handle);

        let response = curiosity.ask().await.unwrap();
        assert!(response.contains("I am content"));
    }
}
