//! The Weaver: A Narrative Connection Generator.
//!
//! "We are all connected. Sometimes you just have to look hard enough."
//!
//! This module uses Vortex inference to hallucinate plausible narrative connections
//! between two arbitrary entities in the knowledge graph.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::{info, instrument};

/// A step in the woven narrative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaveStep {
    /// Description of the connection step.
    pub description: String,
    /// Name of an entity involved in this step (existing or hypothetical).
    pub entity_name: Option<String>,
    /// Type of relationship implied (e.g., "created", "met", "destroyed").
    pub relationship: Option<String>,
}

/// The result of a Weave operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaveResult {
    /// A coherent story connecting the two entities.
    pub narrative: String,
    /// Structured steps of the connection.
    pub steps: Vec<WeaveStep>,
}

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    paper: PsychicPaper,
}

impl Weaver {
    /// Create a new Weaver engine.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            vortex,
            gallifrey,
            paper: PsychicPaper::new(),
        }
    }

    /// Find an entity by name (case-insensitive, exact match).
    ///
    /// This performs a linear scan of the knowledge base, which is inefficient
    /// but acceptable for this experimental feature.
    fn find_entity_by_name(&self, name: &str) -> Option<Entity> {
        let name_lower = name.to_lowercase();
        let result = std::sync::Mutex::new(None);
        let now = Utc::now();

        // We accept the inefficiency for the sake of "The Spark"
        let _ = self.gallifrey.knowledge().scan_history(|history| {
            // Check if we already found it
            if let Ok(guard) = result.lock() {
                if guard.is_some() {
                    return;
                }
            }

            // Find the currently active version of the entity
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(now, now))
            {
                if entity.name.to_lowercase() == name_lower {
                    if let Ok(mut guard) = result.lock() {
                        *guard = Some(entity.clone());
                    }
                }
            }
        });

        result.into_inner().unwrap_or(None)
    }

    /// Weave a narrative connection between two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or inference fails.
    #[instrument(skip(self))]
    pub async fn weave(
        &self,
        source_name: &str,
        target_name: &str,
        model: ModelHandle,
    ) -> ChronosResult<WeaveResult> {
        info!("Weaving connection between '{}' and '{}'", source_name, target_name);

        let source = self
            .find_entity_by_name(source_name)
            .ok_or_else(|| {
                ChronosError::Common(tardis_common::Error::EntityNotFound(format!(
                    "Entity '{source_name}'"
                )))
            })?;

        let target = self
            .find_entity_by_name(target_name)
            .ok_or_else(|| {
                ChronosError::Common(tardis_common::Error::EntityNotFound(format!(
                    "Entity '{target_name}'"
                )))
            })?;

        // Construct context strings
        let source_desc = format!(
            "Name: {}\nType: {}\nProperties: {}",
            source.name,
            source.entity_type,
            serde_json::to_string(&source.properties).unwrap_or_default()
        );

        let target_desc = format!(
            "Name: {}\nType: {}\nProperties: {}",
            target.name,
            target.entity_type,
            serde_json::to_string(&target.properties).unwrap_or_default()
        );

        let prompt = format!(
            "TASK: Create a plausible, creative narrative connection between two seemingly unrelated entities in the system.\n\n\
            ENTITY A:\n{source_desc}\n\n\
            ENTITY B:\n{target_desc}\n\n\
            INSTRUCTIONS:\n\
            1. Invent a chain of events, shared history, or causal link that connects A to B.\n\
            2. You may hypothesize intermediate entities or events.\n\
            3. Be creative but logical within the context of the entity types.\n\
            4. Return a JSON object with:\n\
               - 'narrative': A short paragraph telling the story of the connection.\n\
               - 'steps': A list of objects with 'description', 'entity_name' (optional), and 'relationship' (optional).\n\
            Output ONLY JSON."
        );

        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 1024,
                    temperature: 0.8, // High creativity
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let narrative = parsed
            .get("narrative")
            .and_then(Value::as_str)
            .unwrap_or("The threads are tangled. No clear path found.")
            .to_string();

        let mut steps = Vec::new();
        if let Some(arr) = parsed.get("steps").and_then(Value::as_array) {
            for item in arr {
                let description = item
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown step")
                    .to_string();
                let entity_name = item.get("entity_name").and_then(Value::as_str).map(String::from);
                let relationship = item.get("relationship").and_then(Value::as_str).map(String::from);
                steps.push(WeaveStep {
                    description,
                    entity_name,
                    relationship,
                });
            }
        }

        Ok(WeaveResult { narrative, steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[tokio::test]
    async fn test_weaver_connection() {
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Inference
        vortex.set_mock_inference(Box::new(|_, _, _| {
            let response = json!({
                "narrative": "The Doctor used the Sonic Screwdriver to fix the TARDIS.",
                "steps": [
                    {
                        "description": "Doctor activates device",
                        "entity_name": "Sonic Screwdriver",
                        "relationship": "USES"
                    },
                    {
                        "description": "Device repairs ship",
                        "entity_name": "TARDIS",
                        "relationship": "REPAIRS"
                    }
                ]
            });
            Ok(response.to_string())
        }));

        let gallifrey = Arc::new(Gallifrey::new());

        // Insert Source
        let source = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "The Doctor".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.knowledge().insert_entity(source).unwrap();

        // Insert Target
        let target = Entity {
            id: EntityId::new(),
            entity_type: "Ship".to_string(),
            name: "TARDIS".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.knowledge().insert_entity(target).unwrap();

        let weaver = Weaver::new(vortex, gallifrey);
        let model = ModelHandle::new(1); // Dummy handle

        let result = weaver.weave("The Doctor", "TARDIS", model).await.unwrap();

        assert_eq!(result.narrative, "The Doctor used the Sonic Screwdriver to fix the TARDIS.");
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].entity_name.as_deref(), Some("Sonic Screwdriver"));
    }

    #[tokio::test]
    async fn test_weaver_missing_entity() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
        let weaver = Weaver::new(vortex, gallifrey);
        let model = ModelHandle::new(1);

        let result = weaver.weave("Ghost", "Shadow", model).await;
        assert!(result.is_err());
    }
}
