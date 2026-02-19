//! The Weaver: A Narrative Connection Engine.
//!
//! "Everything is connected. The Weaver just finds the thread."
//!
//! This module uses Vortex (LLM) to generate a narrative connecting two distinct entities
//! based on their shared history in Gallifrey.

use crate::error::{ChronosError, ChronosResult};
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use tardis_common::id::ModelHandle;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::{info, instrument};

/// The Weaver engine.
#[derive(Debug)]
pub struct Weaver {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
}

impl Weaver {
    /// Create a new Weaver.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self { vortex, gallifrey }
    }

    /// Weave a narrative connecting two entities.
    ///
    /// # Errors
    ///
    /// Returns an error if entities are not found or inference fails.
    #[instrument(skip(self))]
    pub async fn weave(
        &self,
        entity_name_1: &str,
        entity_name_2: &str,
        model_handle: Option<ModelHandle>,
    ) -> ChronosResult<String> {
        info!(
            "Weaving narrative between '{entity_name_1}' and '{entity_name_2}'"
        );

        // 1. Find entities and their history
        let history_1 = self.find_history(entity_name_1)?;
        let history_2 = self.find_history(entity_name_2)?;

        if history_1.is_empty() {
            return Err(ChronosError::Common(tardis_common::Error::EntityNotFound(
                format!("Entity not found: {entity_name_1}"),
            )));
        }
        if history_2.is_empty() {
            return Err(ChronosError::Common(tardis_common::Error::EntityNotFound(
                format!("Entity not found: {entity_name_2}"),
            )));
        }

        // 2. Construct Prompt
        let context_str = Self::format_context(entity_name_1, &history_1, entity_name_2, &history_2);

        let prompt = format!(
            "CONTEXT:\n{context_str}\n\nTASK: Write a short, compelling narrative (max 150 words) that connects '{entity_name_1}' and '{entity_name_2}' based on the timeline above. Speculate on their relationship if not explicitly stated. Focus on causality and shared themes."
        );

        // 3. Inference
        // Use the provided model or the first available loaded model
        let model = if let Some(h) = model_handle {
            h
        } else {
            let loaded_models = self.vortex.list_loaded_models();
            loaded_models
                .first()
                .map(|(h, _)| *h)
                .ok_or_else(|| {
                    ChronosError::Common(tardis_common::Error::Internal(
                        "No model loaded".to_string(),
                    ))
                })?
        };

        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 256,
                    temperature: 0.8,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        Ok(response)
    }

    fn find_history(&self, name: &str) -> ChronosResult<Vec<Entity>> {
        // Since Gallifrey lacks a name index, we scan history.
        // In a real system, we'd use an index or search.
        let result: Arc<Mutex<Vec<Entity>>> = Arc::new(Mutex::new(Vec::new()));
        let result_clone = result.clone();
        let name_lower = name.to_lowercase();

        self.gallifrey
            .knowledge()
            .scan_history(move |history| {
                // Check if any version of the entity matches the name
                // history is a Vec<Entity> representing one entity's history
                if let Some(first) = history.first() {
                     if first.name.to_lowercase() == name_lower {
                        if let Ok(mut lock) = result_clone.lock() {
                            lock.extend(history.iter().cloned());
                        }
                     }
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let mut history = result
            .lock()
            .map_err(|_| ChronosError::Common(tardis_common::Error::Internal("Mutex poisoned".to_string())))?
            .clone();
        // Sort by valid time
        history.sort_by(|a, b| a.temporal.valid_time.start.cmp(&b.temporal.valid_time.start));

        Ok(history)
    }

    fn format_context(
        name_1: &str,
        history_1: &[Entity],
        name_2: &str,
        history_2: &[Entity],
    ) -> String {
        let mut output = String::new();

        let _ = writeln!(output, "--- Entity A: {name_1} ---");
        for e in history_1 {
            let _ = writeln!(output,
                "[{}] {}: {}",
                e.temporal.valid_time.start,
                e.entity_type,
                serde_json::to_string(&e.properties).unwrap_or_default()
            );
        }

        let _ = writeln!(output, "\n--- Entity B: {name_2} ---");
        for e in history_2 {
            let _ = writeln!(output,
                "[{}] {}: {}",
                e.temporal.valid_time.start,
                e.entity_type,
                serde_json::to_string(&e.properties).unwrap_or_default()
            );
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[tokio::test]
    async fn test_weaver_weave() {
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Inference
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("Alice and Bob met at the cafe.".to_string())
        }));

        let gallifrey = Arc::new(Gallifrey::new());

        // Insert Entity 1
        let entity1 = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Alice".to_string(),
            properties: HashMap::from([("role".to_string(), serde_json::json!("Engineer"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(entity1).await.unwrap();

        // Insert Entity 2
        let entity2 = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Bob".to_string(),
            properties: HashMap::from([("role".to_string(), serde_json::json!("Designer"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(entity2).await.unwrap();

        let weaver = Weaver::new(vortex, gallifrey);

        // Use a dummy model handle (id=0)
        let model_handle = ModelHandle::new(0);

        let result = weaver.weave("Alice", "Bob", Some(model_handle)).await;

        assert!(result.is_ok(), "Weave failed: {:?}", result.err());
        let narrative = result.unwrap();
        assert_eq!(narrative, "Alice and Bob met at the cafe.");
    }

    #[tokio::test]
    async fn test_weaver_not_found() {
         let vortex = Arc::new(Vortex::new().unwrap());
         let gallifrey = Arc::new(Gallifrey::new());
         let weaver = Weaver::new(vortex, gallifrey);
         let model_handle = ModelHandle::new(0);

         let result = weaver.weave("Nobody", "Noone", Some(model_handle)).await;
         assert!(result.is_err());
    }
}
