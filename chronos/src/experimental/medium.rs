//! The Medium: Seance Engine.
//!
//! "I can hear the voices of the past."
//!
//! This module allows querying the system state at a specific point in the past.

use crate::error::{ChronosError, ChronosResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::{info, instrument};

/// The Medium engine.
#[derive(Debug)]
pub struct Medium {
    gallifrey: Arc<Gallifrey>,
    vortex: Arc<Vortex>,
    model: ModelHandle,
}

impl Medium {
    /// Create a new Medium engine.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>, vortex: Arc<Vortex>, model: ModelHandle) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Summon the past state and answer a query.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails.
    #[instrument(skip(self))]
    pub async fn summon(&self, query: &str, time: DateTime<Utc>) -> ChronosResult<String> {
        info!("Medium: Summoning past state at {}", time);

        // 1. Gather context from the past
        let context = self.gather_context(query, time)?;

        if context.trim().is_empty() {
            return Ok(
                "The spirits are silent. No relevant memories found from that time.".to_string(),
            );
        }

        // 2. Prompt LLM
        let prompt = format!(
            "<s>[INST] You are the Spirit of the Past, representing the system state as it was on {time}.\n\
             Your knowledge is strictly limited to the following context recovered from that time:\n\n\
             {context}\n\n\
             User Question: \"{query}\"\n\n\
             Answer the question based ONLY on the provided context. If the answer is not in the context, say so fantastically (e.g., \"The mists of time obscure this detail\").\n\
             Do not hallucinate facts not present in the context.[/INST]"
        );

        let params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(512);

        let response = self
            .vortex
            .infer(self.model, &prompt, params)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        Ok(response)
    }

    #[allow(clippy::format_push_string)]
    fn gather_context(&self, query: &str, time: DateTime<Utc>) -> ChronosResult<String> {
        let mut context = String::new();
        let knowledge = self.gallifrey.knowledge();
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        // Scan history for relevant entities active at `time`
        // We use a simple heuristic: check if entity name contains any query word (if word len > 3)
        // or if query contains entity name.
        knowledge
            .scan_history(|history| {
                // Find version active at `time` (using transaction time = time for "as known then")
                if let Some(entity) = history.iter().find(|e| e.temporal.active_at(time, time)) {
                    let name_lower = entity.name.to_lowercase();

                    let relevant = query_lower.contains(&name_lower)
                        || query_words
                            .iter()
                            .any(|w| w.len() > 3 && name_lower.contains(w));

                    if relevant {
                        context.push_str(&format!(
                            "- Entity: {} ({})\n",
                            entity.name, entity.entity_type
                        ));
                        context.push_str(&format!("  Properties: {:?}\n", entity.properties));
                    }
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        Ok(context)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use tardis_gallifrey::domain::Entity;
    use tardis_vortex::{ModelHandle, ModelLoadConfig};

    fn create_test_entity(name: &str, entity_type: &str, time: DateTime<Utc>) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            // Valid from `time` onwards
            temporal: BiTemporalInterval::with_valid_time(TimeRange::starting_at(time)),
            source: Some("test".to_string()),
        }
    }

    #[tokio::test]
    async fn test_medium_summon() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Setup:
        // Entity A created at T1
        // Entity B created at T2
        // Query at T1 should see A but not B.

        let t1 = Utc.with_ymd_and_hms(2023, 10, 27, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2023, 10, 27, 12, 0, 0).unwrap();

        let mut e1 = create_test_entity("OldProject", "Project", t1);
        // Backdate transaction time to T1
        e1.temporal.transaction_time.start = t1;
        gallifrey.insert(e1).await.unwrap();

        let mut e2 = create_test_entity("NewProject", "Project", t2);
        // Backdate transaction time to T2
        e2.temporal.transaction_time.start = t2;
        gallifrey.insert(e2).await.unwrap();

        // Mock Vortex
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("- Entity: OldProject") && !prompt.contains("- Entity: NewProject") {
                Ok("I see OldProject.".to_string())
            } else if prompt.contains("- Entity: NewProject") {
                Ok("I see NewProject.".to_string())
            } else {
                Ok("I see nothing.".to_string())
            }
        }));

        vortex.set_mock_load_model(Box::new(|_, _| Ok(ModelHandle::new(1))));

        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();
        let medium = Medium::new(gallifrey, vortex, handle);

        // Summon at T1 + 1 minute (should see OldProject, but NOT NewProject)
        let query_time = t1 + chrono::Duration::minutes(1);

        let response = medium
            .summon("OldProject NewProject", query_time)
            .await
            .unwrap();

        // The LLM mock returns "I see OldProject." if NewProject is NOT in context.
        assert_eq!(response, "I see OldProject.");
    }
}
