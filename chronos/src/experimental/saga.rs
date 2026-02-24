//! The Saga: Narrative Pathfinder.
//!
//! "Every journey has a story."
//!
//! This module combines the pathfinding capabilities of the Astrolabe with the
//! narrative generation of the Weaver to tell the story of how two entities are connected.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::astrolabe::Astrolabe;
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_common::llm::InferenceParams;
use tardis_common::traits::LlmService;
use tardis_gallifrey::Gallifrey;
use tracing::{info, instrument};

/// The Saga engine.
#[derive(Debug)]
pub struct Saga {
    gallifrey: Arc<Gallifrey>,
    llm: Arc<dyn LlmService>,
    model: Option<ModelHandle>,
}

impl Saga {
    /// Create a new Saga engine.
    #[must_use]
    pub fn new(
        gallifrey: Arc<Gallifrey>,
        llm: Arc<dyn LlmService>,
        model: Option<ModelHandle>,
    ) -> Self {
        Self {
            gallifrey,
            llm,
            model,
        }
    }

    /// Tell the tale of the connection between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if pathfinding fails or the story cannot be told.
    #[instrument(skip(self))]
    pub async fn tell_tale(&self, start_name: &str, end_name: &str) -> ChronosResult<String> {
        info!(
            "Saga: Weaving a tale from '{}' to '{}'",
            start_name, end_name
        );

        // 1. Find the path using Astrolabe
        let astrolabe = Astrolabe::new(self.gallifrey.clone());
        let path = astrolabe.navigate(start_name, end_name)?;

        if path.is_empty() {
            return Ok("The void separates them. No path found.".to_string());
        }

        // 2. Construct the narrative prompt
        let mut path_desc = String::new();
        for (i, segment) in path.iter().enumerate() {
            if i > 0 {
                let via = segment.via.as_deref().unwrap_or("leads to");
                writeln!(path_desc, "  ↓ [{}]", via).map_err(|e| {
                    ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
                })?;
            }
            writeln!(
                path_desc,
                "- {} ({})",
                segment.entity.name, segment.entity.entity_type
            )
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;
        }

        let prompt = format!(
            "<s>[INST] You are The Bard of the System, a storyteller who narrates the hidden connections within the OS.\n\
             Your task is to tell a short, creative story (max 3 sentences) explaining the causal chain between these entities based on the path provided.\n\
             \n\
             The Path:\n\
             {}\n\
             \n\
             Narrate the journey from start to end. Be dramatic but logical.[/INST]",
            path_desc
        );

        // 3. Generate the story
        let mut params = InferenceParams::default()
            .with_temperature(0.8)
            .with_max_tokens(200);

        if let Some(model) = self.model {
            params = params.with_model(model);
        }

        let story = self
            .llm
            .infer(&prompt, params)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        Ok(story)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::any::Any;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::{Entity, Relationship};

    // Mock LlmService
    #[derive(Debug)]
    struct MockLlm;

    #[async_trait]
    impl LlmService for MockLlm {
        async fn infer(
            &self,
            prompt: &str,
            _params: InferenceParams,
        ) -> tardis_common::Result<String> {
            Ok(format!("Mock story based on: {}", prompt))
        }

        async fn embed(&self, _text: &str) -> tardis_common::Result<Vec<f32>> {
            Ok(vec![0.0; 384])
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn create_test_entity(name: &str, embedding: Option<Vec<f32>>) -> Entity {
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

    #[tokio::test]
    async fn test_saga_tell_tale() {
        let gallifrey = Arc::new(Gallifrey::new());
        let llm = Arc::new(MockLlm);

        // Setup graph: Start -> End
        let e1 = create_test_entity("Start", Some(vec![1.0, 0.0]));
        let e2 = create_test_entity("End", Some(vec![0.9, 0.1])); // Close enough for Astrolabe

        let id1 = e1.id;
        let id2 = e2.id;

        gallifrey.insert(e1).await.unwrap();
        gallifrey.insert(e2).await.unwrap();

        gallifrey
            .knowledge()
            .insert_relationship(Relationship {
                id: EntityId::new(),
                relationship_type: "CAUSED".to_string(),
                source: id1,
                target: id2,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            })
            .unwrap();

        let saga = Saga::new(gallifrey, llm, None);
        let story = saga.tell_tale("Start", "End").await.unwrap();

        assert!(story.contains("Mock story based on"));
        assert!(story.contains("The Path:"));
        assert!(story.contains("Start"));
        assert!(story.contains("End"));
    }
}
