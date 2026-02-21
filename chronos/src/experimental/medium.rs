//! The Medium 🔮
//!
//! "I see dead people... well, deprecated entities."
//!
//! A module that allows you to "channel" a past version of an entity, interacting with it
//! as if it were a chatbot. It reconstructs the entity's state at a specific point in time
//! and uses the LLM to roleplay as that entity.

use crate::error::{ChronosError, ChronosResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_common::Error;
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

/// A session with a summoned entity.
#[derive(Debug)]
pub struct MediumSession {
    /// The summoned entity snapshot.
    pub entity: Entity,
    /// The time at which it was summoned.
    pub time: DateTime<Utc>,
    /// The LLM engine.
    vortex: Arc<Vortex>,
    /// The model handle.
    model: ModelHandle,
    /// Conversation history for this session (User, AI).
    history: Vec<(String, String)>,
}

impl Medium {
    /// Create a new Medium instance.
    #[must_use]
    pub const fn new(
        gallifrey: Arc<Gallifrey>,
        vortex: Arc<Vortex>,
        model: ModelHandle,
    ) -> Self {
        Self {
            gallifrey,
            vortex,
            model,
        }
    }

    /// Summon an entity from the past.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity cannot be found at the specified time.
    pub fn summon(
        &self,
        entity_name: &str,
        time: DateTime<Utc>,
    ) -> ChronosResult<MediumSession> {
        // 1. Find the entity ID
        let id = self.find_entity_id_at(entity_name, time)?;

        // 2. Retrieve the entity state at that time
        // We want: Valid at `time` AND Known at `time` (Transaction Time).
        // This simulates "what we knew then".
        let entity = self
            .gallifrey
            .knowledge()
            .get_entity_at(id, time, time)
            .map_err(|e| ChronosError::Common(e.into()))?
            .ok_or_else(|| {
                ChronosError::Common(Error::EntityNotFound(format!(
                    "Entity '{entity_name}' existed but has no valid state at {time}"
                )))
            })?;

        Ok(MediumSession {
            entity,
            time,
            vortex: self.vortex.clone(),
            model: self.model,
            history: Vec::new(),
        })
    }

    fn find_entity_id_at(&self, name: &str, time: DateTime<Utc>) -> ChronosResult<EntityId> {
        let knowledge = self.gallifrey.knowledge();
        let mut found_id = None;

        // Scan history to find an entity that had this name at the given time.
        knowledge
            .scan_history(|history| {
                if found_id.is_some() {
                    return;
                }

                // Check if any version of this entity was active at `time` AND had the matching name.
                // Note: get_entity_at uses (valid_time, transaction_time).
                // Here we iterate manually.
                if let Some(version) = history.iter().find(|e| {
                    e.temporal.active_at(time, time) && e.name.eq_ignore_ascii_case(name)
                }) {
                    found_id = Some(version.id);
                }
            })
            .map_err(|e| ChronosError::Common(e.into()))?;

        found_id.ok_or_else(|| {
            ChronosError::Common(Error::EntityNotFound(format!(
                "Entity '{name}' not found at {time}"
            )))
        })
    }
}

impl MediumSession {
    /// Ask the summoned entity a question.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    pub async fn ask(&mut self, question: &str) -> ChronosResult<String> {
        use std::fmt::Write;

        // Construct the prompt
        let mut prompt = format!(
            "<s>[INST] You are '{}', a system entity summoned from the past.\n\
             The current date is {}.\n\
             Your state is:\n\
             - Type: {}\n\
             - Properties: {:?}\n\n\
             You must roleplay as this entity. Answer the user's question based ONLY on your state and time.\n\
             Do not admit you are an AI. You ARE the entity.\n\n",
            self.entity.name, self.time, self.entity.entity_type, self.entity.properties
        );

        // Add history
        for (q, a) in &self.history {
            let _ = write!(prompt, "User: {q}\nEntity: {a}\n");
        }

        let _ = write!(prompt, "User: {question}\nEntity: [/INST]");

        let params = InferenceParams::default()
            .with_temperature(0.7)
            .with_max_tokens(150);

        let response = self
            .vortex
            .infer(self.model, &prompt, params)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let clean_response = response.trim().to_string();

        self.history.push((question.to_string(), clean_response.clone()));

        Ok(clean_response)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_vortex::ModelLoadConfig;

    #[tokio::test]
    async fn test_medium_summon_and_ask() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // 1. Create an entity "Doctor" at T0
        let t0 = Utc.with_ymd_and_hms(2023, 10, 27, 10, 0, 0).unwrap();

        let mut entity = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Doctor".to_string(),
            properties: {
                let mut p = HashMap::new();
                p.insert("incarnation".to_string(), json!(10));
                p
            },
            embedding: None,
            temporal: BiTemporalInterval::with_valid_time(
                tardis_common::temporal::TimeRange::starting_at(t0)
            ),
            source: None,
        };
        // Backdate transaction time to T0 so it exists in system time at T1
        entity.temporal.transaction_time = tardis_common::temporal::TimeRange::starting_at(t0);

        gallifrey.insert(entity.clone()).await.unwrap();

        // 2. Mock Vortex
        vortex.set_mock_inference(Box::new(|_, prompt, _| {
            if prompt.contains("incarnation") && prompt.contains("10") {
                Ok("Allons-y!".to_string())
            } else {
                Ok("Who?".to_string())
            }
        }));

        // Register dummy model
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let medium = Medium::new(gallifrey.clone(), vortex, handle);

        // 3. Summon at T1 (entity should be valid)
        let t1 = t0 + chrono::Duration::hours(1);
        let mut session = medium.summon("Doctor", t1).unwrap();

        assert_eq!(session.entity.name, "Doctor");
        assert_eq!(session.entity.properties.get("incarnation"), Some(&json!(10)));

        // 4. Ask
        let response = session.ask("Ready?").await.unwrap();
        assert_eq!(response, "Allons-y!");
    }

    #[tokio::test]
    async fn test_medium_summon_fails_before_creation() {
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        let t0 = Utc.with_ymd_and_hms(2023, 10, 27, 10, 0, 0).unwrap();

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "Companion".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::with_valid_time(
                tardis_common::temporal::TimeRange::starting_at(t0)
            ),
            source: None,
        };
        gallifrey.insert(entity).await.unwrap();

        // Register dummy model
        vortex.set_mock_load_model(Box::new(move |_, _| Ok(ModelHandle::new(1))));
        let handle = vortex
            .load_model("dummy", ModelLoadConfig::default())
            .await
            .unwrap();

        let medium = Medium::new(gallifrey.clone(), vortex, handle);

        // Try to summon before T0
        let t_before = t0 - chrono::Duration::hours(1);
        let result = medium.summon("Companion", t_before);

        assert!(result.is_err());
    }
}
