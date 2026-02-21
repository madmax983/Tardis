//! The Medium 🔮
//!
//! A module that allows communicating with the system's past state.
//!
//! "The Medium" reconstructs the knowledge graph as it existed at a specific point in time
//! and uses the LLM to roleplay as the system at that moment.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};

/// The Medium engine.
#[derive(Debug)]
pub struct Medium {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl Medium {
    /// Create a new Medium instance.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>, model: ModelHandle) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Summon the spirit of the system from a specific time.
    ///
    /// # Errors
    ///
    /// Returns an error if the knowledge graph cannot be scanned or inference fails.
    pub async fn summon(&self, target_time: DateTime<Utc>, query: &str) -> Result<String> {
        let context = self.gather_context(target_time)?;

        if context.is_empty() {
            return Ok(format!(
                "The void is silent. I have no memories from {target_time}."
            ));
        }

        let context_str = context
            .iter()
            .map(|e| {
                format!(
                    "- {} ({})\n  Properties: {:?}",
                    e.name, e.entity_type, e.properties
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "<s>[INST] You are the Tardis OS. The current time is {target_time}.
You are speaking to a user from the future.
You ONLY know what is in your knowledge base at this time.
Do not hallucinate facts not present in the context.

Your Knowledge Base at {target_time}:
{context_str}

User Query: {query}

Response: [/INST]",
        );

        let params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(200);

        let result = self
            .vortex
            .infer(self.model, &prompt, params)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(result)
    }

    fn gather_context(&self, target_time: DateTime<Utc>) -> Result<Vec<Entity>> {
        let knowledge = self.gallifrey.knowledge();
        let mut context = Vec::new();

        // Scan history to find entities active at target_time
        // We use target_time for both valid and transaction time to simulate
        // what the system "knew" at that moment.
        knowledge.scan_history(|history| {
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(target_time, target_time))
            {
                context.push(entity.clone());
            }
        })?;

        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: Some("test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_gather_context() {
        let gallifrey = Arc::new(Gallifrey::new());
        // We can't mock Vortex easily, so we just test the context gathering logic
        // by creating a dummy Medium with a dummy Vortex (which we can't do easily in unit tests
        // without a lot of mocking).
        // Instead, let's just test the logic directly using Gallifrey.

        let entity = create_test_entity("Past Entity");
        gallifrey.insert(entity.clone()).await.unwrap(); // Takes standard lock

        // We need to wait a bit or manipulate time, but BiTemporalInterval::now() uses Utc::now().
        // Since we can't easily inject time into Gallifrey, we'll just test that it finds it "now".

        let knowledge = gallifrey.knowledge();
        let mut context = Vec::new();
        let target_time = Utc::now();

        knowledge.scan_history(|history| {
            if let Some(e) = history.iter().find(|e| e.temporal.active_at(target_time, target_time)) {
                context.push(e.clone());
            }
        }).unwrap();

        assert_eq!(context.len(), 1);
        assert_eq!(context[0].name, "Past Entity");
    }
}
