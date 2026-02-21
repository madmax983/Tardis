//! The Medium 🔮
//!
//! A module for summoning the "spirits" of entities from the past.
//! It reconstructs an entity's state at a specific point in time and uses
//! the LLM to generate a persona that can answer questions about its existence.

use crate::error::ChronosResult;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tardis_common::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Medium service.
#[derive(Debug)]
pub struct Medium {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model_handle: ModelHandle,
}

impl Medium {
    /// Create a new Medium instance.
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

    /// Summon an entity from a specific time.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity cannot be found or if the time is invalid.
    pub async fn summon(
        &self,
        entity_name: &str,
        timestamp: DateTime<Utc>,
    ) -> ChronosResult<Option<MediumSession>> {
        // Find entity ID by name (linear scan)
        let mut target_id = None;

        // We use a linear scan because Gallifrey doesn't index names yet.
        // This is acceptable for an experimental feature.
        let _ = self.gallifrey.knowledge().scan_history(|history| {
            if let Some(first) = history.first() {
                if first.name == entity_name {
                    target_id = Some(first.id);
                }
            }
        });

        let Some(id) = target_id else {
            return Ok(None);
        };

        // Get history and find the version active at timestamp
        let history = self.gallifrey.get_history(id).await.map_err(|e| {
            crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
        })?;

        let snapshot = history
            .into_iter()
            .find(|e| e.temporal.active_at(timestamp, timestamp));

        match snapshot {
            Some(entity) => Ok(Some(MediumSession {
                entity,
                vortex: Arc::clone(&self.vortex),
                model_handle: self.model_handle,
                timestamp,
            })),
            None => Ok(None),
        }
    }
}

/// A session with a summoned entity.
#[derive(Debug)]
pub struct MediumSession {
    entity: Entity,
    vortex: Arc<Vortex>,
    model_handle: ModelHandle,
    timestamp: DateTime<Utc>,
}

impl MediumSession {
    /// Ask the entity a question.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM inference fails.
    pub async fn ask(&self, question: &str) -> ChronosResult<String> {
        let props = serde_json::to_string_pretty(&self.entity.properties).unwrap_or_default();

        let prompt = format!(
            "You are the spirit of '{name}', summoned from {time}.\n\
             Your existence at that time was defined by these properties:\n{props}\n\n\
             User Question: {question}\n\n\
             Answer as the entity '{name}' would, based on your state at that time. \
             Be slightly dramatic and mysterious.",
            name = self.entity.name,
            time = self.timestamp,
            props = props,
            question = question
        );

        let params = InferenceParams {
            max_tokens: 150,
            temperature: 0.8,
            ..InferenceParams::default()
        };

        self.vortex
            .infer(self.model_handle, &prompt, params)
            .await
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[tokio::test]
    async fn test_medium_summon() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Vortex
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("I am the ghost of systems past...".to_string())
        }));

        let handle = ModelHandle::new(1);
        let medium = Medium::new(Arc::clone(&gallifrey), vortex, handle);

        // Create a past entity
        let id = EntityId::new();
        let past_time = Utc::now() - chrono::Duration::days(365);

        let mut entity = Entity {
            id,
            entity_type: "SystemProcess".to_string(),
            name: "OldProcess".to_string(),
            properties: HashMap::from([("status".to_string(), serde_json::json!("Dead"))]),
            embedding: None,
            temporal: BiTemporalInterval::now(), // Temporarily now
            source: None,
        };
        // Manually adjust temporal to be valid in the past
        entity.temporal.valid_time.start = past_time;
        entity.temporal.valid_time.end = Some(past_time + chrono::Duration::days(1));
        // Also adjust transaction time so it appears we knew about it back then
        entity.temporal.transaction_time.start = past_time;

        gallifrey.insert(entity).await.unwrap();

        // Summon it
        let session = medium
            .summon("OldProcess", past_time + chrono::Duration::hours(1))
            .await
            .unwrap();
        assert!(session.is_some());

        let response = session.unwrap().ask("Who are you?").await.unwrap();
        assert_eq!(response, "I am the ghost of systems past...");
    }
}
