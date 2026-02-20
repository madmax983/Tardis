//! The Medium: Talk to the ghosts of the system.
//!
//! This module allows interacting with a past version of the system state.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Medium allows interacting with a past version of the system state.
#[derive(Debug)]
pub struct Medium {
    vortex: Arc<Vortex>,
    model_handle: ModelHandle,
    context: String,
    timestamp: DateTime<Utc>,
}

impl Medium {
    /// Summon the system state from a specific time.
    ///
    /// # Errors
    ///
    /// Returns an error if history scanning fails.
    pub fn summon(
        gallifrey: &Gallifrey,
        vortex: Arc<Vortex>,
        model_handle: ModelHandle,
        timestamp: DateTime<Utc>,
    ) -> Result<Self> {
        let mut context = format!("System State as of {}:\n\n", timestamp);
        let mut entity_count = 0;

        // Scan history to find active entities
        gallifrey.knowledge().scan_history(|history| {
            // Find the version active at both valid_time and transaction_time = timestamp
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(timestamp, timestamp))
            {
                if entity_count < 50 {
                    context.push_str(&format!(
                        "- {}: {} (ID: {})\n",
                        entity.name, entity.entity_type, entity.id
                    ));
                    entity_count += 1;
                }
            }
        })?;

        if entity_count == 0 {
            context.push_str("(No active entities found at this time)\n");
        }

        Ok(Self {
            vortex,
            model_handle,
            context,
            timestamp,
        })
    }

    /// Ask the medium a question.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    pub async fn ask(&self, question: &str) -> Result<String> {
        let prompt = format!(
            "You are the Tardis OS as it existed at {}.
Your knowledge is limited to the following facts:
{}

User: {}
System:",
            self.timestamp, self.context, question
        );

        let params = InferenceParams::default()
            .with_max_tokens(150)
            .with_temperature(0.7);

        self.vortex
            .infer(self.model_handle, &prompt, params)
            .await
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Entity;

    #[test]
    fn test_summon_constructs_context() {
        let gallifrey = Gallifrey::new();

        // Populate gallifrey with some data
        let id = EntityId::new();
        let entity = Entity {
            id,
            entity_type: "Test".to_string(),
            name: "Ghost".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };

        gallifrey.knowledge().insert_entity(entity).unwrap();

        // Query slightly in the future to ensure we are inside the interval
        // (insert_entity sets transaction time to now, valid time to now)
        std::thread::sleep(std::time::Duration::from_millis(10));
        let timestamp = Utc::now();

        let vortex = Arc::new(Vortex::new().unwrap());
        let handle = ModelHandle::new(42);

        let medium = Medium::summon(&gallifrey, vortex, handle, timestamp).unwrap();

        // Verify context contains the entity
        assert!(medium.context.contains("Ghost"));
        assert!(medium.context.contains("Test"));
        assert!(medium.context.contains(&id.to_string()));
    }
}
