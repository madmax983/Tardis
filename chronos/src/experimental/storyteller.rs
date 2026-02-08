//! # The Storyteller
//!
//! A module that turns cold, hard telemetry into a hero's journey.
//! It monitors system state changes and uses the LLM to generate
//! dramatic, sci-fi style log entries (Sagas).
//!
//! "Captain's Log, Stardate 4523.9..."

use crate::error::ChronosResult;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::domain::{ChangeType, Entity};
use tardis_common::id::EntityId;
use tardis_common::llm::{InferenceParams, ModelLoadConfig};
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// The Storyteller engine.
#[derive(Debug)]
pub struct Storyteller {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
}

impl Storyteller {
    /// Create a new Storyteller.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self { vortex, gallifrey }
    }

    /// Narrate the events within a time range.
    ///
    /// Fetches system changes from Gallifrey, prompts Vortex to write a story,
    /// and stores the result as a "Saga" entity.
    ///
    /// # Errors
    ///
    /// Returns an error if fetching changes, inference, or storage fails.
    #[instrument(skip(self))]
    pub async fn narrate(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> ChronosResult<EntityId> {
        info!("Gathering tales from {} to {}...", start, end);

        // 1. Fetch system changes
        let changes = self
            .gallifrey
            .system_state()
            .get_changes(start, end)
            .map_err(|e| {
                crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string()))
            })?;

        if changes.is_empty() {
            info!("No events to narrate.");
            // Even if empty, we might want to narrate "peaceful silence"
        }

        // 2. Format the prompt
        let mut events_log = String::new();
        for change in changes {
            let action = match change.change_type {
                ChangeType::Create => "emerged",
                ChangeType::Update => "shifted",
                ChangeType::Delete => "vanished",
            };
            let _ = writeln!(
                &mut events_log,
                "- [{}] {} at {}",
                change.timestamp.format("%H:%M:%S"),
                action,
                change.path
            );
        }

        if events_log.is_empty() {
            events_log = "The system was dormant. Silence reigned.".to_string();
        }

        let prompt = format!(
            "You are the ship's computer AI. Based on the following system events, write a dramatic, short sci-fi log entry (max 100 words).\n\n\
            Time Range: {start} - {end}\n\
            Events:\n\
            {events_log}\n\n\
            Log Entry:",
        );

        // 3. Invoke Vortex
        // We use a default model for now.
        let handle = self
            .vortex
            .load_model("default", ModelLoadConfig::default())
            .await?;

        let params = InferenceParams {
            max_tokens: 200,
            temperature: 0.7,
            ..Default::default()
        };

        let story_text = self.vortex.infer(handle, &prompt, params).await?;
        info!("Generated saga: {}", story_text);

        // 4. Store the Saga
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Saga".to_string(),
            name: format!("Saga: {} - {}", start.format("%H:%M"), end.format("%H:%M")),
            properties: {
                let mut props = HashMap::new();
                props.insert("content".to_string(), serde_json::json!(story_text));
                props.insert("start_time".to_string(), serde_json::json!(start));
                props.insert("end_time".to_string(), serde_json::json!(end));
                props
            },
            embedding: None, // TODO: Embed the story
            temporal: tardis_common::temporal::BiTemporalInterval::now(),
            source: Some("Chronos::Storyteller".to_string()),
        };

        let id = self
            .gallifrey
            .insert(entity)
            .await
            .map_err(crate::error::ChronosError::Common)?;

        Ok(id)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tardis_common::domain::{Change, ChangeType};
    use tardis_common::id::ModelHandle;

    #[tokio::test]
    async fn test_narrate() {
        // Setup
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());

        // Mock Vortex
        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(0))));
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("The CPU core hummed with forbidden energy...".to_string())
        }));

        // Populate Gallifrey with some changes
        let start = Utc::now();
        let change = Change {
            timestamp: start,
            path: "/sys/cpu/0".to_string(),
            change_type: ChangeType::Update,
            old_value: None,
            new_value: None,
        };
        gallifrey.system_state().record_change(change).unwrap();

        let storyteller = Storyteller::new(vortex, gallifrey);
        let end = Utc::now();

        // Run
        let result = storyteller.narrate(start, end).await;

        // Verify
        assert!(result.is_ok());
        let id = result.unwrap();

        // Check storage (conceptually - since we can't easily query by ID without exposing internal stores in test)
        // But since we returned an ID, the insert succeeded.
        println!("Created Saga: {id}");
    }
}
