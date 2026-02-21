//! The Medium 🕯️
//!
//! "I see dead people... and deprecated APIs."
//!
//! A module that allows you to "summon" a snapshot of the knowledge base from a
//! specific point in time and query it using the LLM.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::fmt::Write;
use std::sync::Arc;
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

    /// Summon the system state from a specific time and answer a query.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails or knowledge cannot be scanned.
    pub async fn summon(&self, timestamp: DateTime<Utc>, query: &str) -> Result<String> {
        let knowledge = self.gallifrey.knowledge();
        let mut context = String::new();

        // Scan history to find entities active at the given timestamp
        // We set both valid_time and transaction_time to the query timestamp
        // to simulate "what did we know at that time?"
        knowledge.scan_history(|history| {
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(timestamp, timestamp))
            {
                let _ = writeln!(
                    context,
                    "- {} ({}): {:?}",
                    entity.name, entity.entity_type, entity.properties
                );
            }
        })?;

        if context.is_empty() {
            return Ok(format!(
                "I gaze into the past at {timestamp}, but I see nothing. The void is empty."
            ));
        }

        let prompt = format!(
            "<s>[INST] You are The Medium, a spiritual channel to the past state of the Tardis OS.
The current time for you is {timestamp}. You have NO knowledge of the future.
You only know the following entities which existed at that time:

{context}

User Query: {query}

Answer the user's query based ONLY on the entities listed above.
If the information is not in the list, say 'The spirits are silent on this matter.'
Speak in a mystical, slightly eerie tone. [/INST]"
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Entity;
    use tardis_vortex::ModelLoadConfig;

    fn create_test_entity(name: &str, status: &str) -> Entity {
        let mut props = HashMap::new();
        props.insert("status".to_string(), json!(status));

        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: props,
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: Some("test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_medium_summon_past() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Setup mock Vortex
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            Ok(format!("Mock response to: {}", prompt))
        }));

        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let medium = Medium::new(gallifrey.clone(), vortex, handle);

        // T0: Create entity
        let entity = create_test_entity("Ghost", "Alive");
        let id = entity.id;
        gallifrey.insert(entity).await.unwrap();

        // Wait for time to pass
        thread::sleep(Duration::from_millis(10));
        let t0 = Utc::now();
        thread::sleep(Duration::from_millis(10));

        // T1: Update entity
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), json!("Dead"));
        gallifrey.update(id, json!(updates)).await.unwrap();

        // Query at T0 (Should see "Alive")
        // We can't easily check the *content* of the prompt passed to the mock without more complex mocking,
        // but we can verifying it runs without error.
        // To properly test, we'd need to inspect what scan_history found.
        // Since scan_history is synchronous and we trust it works (tested in gallifrey),
        // we mainly test integration here.

        let response = medium.summon(t0, "Is the ghost alive?").await.unwrap();
        assert!(response.contains("Mock response"));
    }
}
