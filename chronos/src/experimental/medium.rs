//! The Medium: Channeling the Past.
//!
//! "I see dead people... well, deprecated entities."
//!
//! This module allows users to "summon" a specific entity as it existed at a specific point in time.
//! It constructs a persona based on the entity's state at that time and allows for a conversation.

use crate::error::{ChronosError, ChronosResult};
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use tardis_common::id::ModelHandle;
use tardis_gallifrey::Gallifrey;
use tracing::{info, instrument};

/// The result of a successful summoning.
#[derive(Debug)]
pub struct SeanceSession {
    /// The name of the summoned entity.
    pub entity_name: String,
    /// The time at which the entity is being channeled.
    pub time: DateTime<Utc>,
    /// The constructed system prompt defining the persona.
    pub system_prompt: String,
    /// The handle to the loaded model to use.
    pub model_handle: ModelHandle,
}

/// The Medium: A bridge to the past.
#[derive(Debug)]
pub struct Medium {
    gallifrey: Arc<Gallifrey>,
}

impl Medium {
    /// Create a new Medium.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Summon an entity from the past.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity cannot be found or no model is loaded.
    #[instrument(skip(self))]
    pub fn summon(
        &self,
        entity_name: &str,
        time: DateTime<Utc>,
        model_handle: ModelHandle,
    ) -> ChronosResult<SeanceSession> {
        info!("Summoning {} from {}", entity_name, time);

        // 1. Scan history to find the entity active at the given time.
        let entity_cell = Mutex::new(None);
        let target_name = entity_name.to_lowercase();

        self.gallifrey
            .knowledge()
            .scan_history(|history| {
                // Optimization: if we already found it, stop checking (best effort, scan_history continues anyway)
                if entity_cell.lock().unwrap().is_some() {
                    return;
                }

                if let Some(entity) = history.iter().find(|e| {
                    e.name.to_lowercase() == target_name && e.temporal.active_at(time, time)
                }) {
                    *entity_cell.lock().unwrap() = Some(entity.clone());
                }
            })
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let entity = entity_cell
            .into_inner()
            .unwrap()
            .ok_or_else(|| ChronosError::Common(tardis_common::Error::EntityNotFound(format!(
                "Entity '{}' not found active at {}",
                entity_name, time
            ))))?;

        // 2. Construct the persona.
        let properties_json = serde_json::to_string_pretty(&entity.properties).unwrap_or_default();

        let system_prompt = format!(
            "You are {name}, an entity within the system.\n\
            Current Time: {time}\n\
            Your State: {properties}\n\
            \n\
            INSTRUCTIONS:\n\
            - You are roleplaying as this specific entity at this specific time.\n\
            - You DO NOT know about events that happen after {time}.\n\
            - Respond in character based on your properties and type ({type}).\n\
            - If asked about the future, express ignorance or make predictions based on your current state.",
            name = entity.name,
            time = time.to_rfc3339(),
            properties = properties_json,
            type = entity.entity_type
        );

        Ok(SeanceSession {
            entity_name: entity.name,
            time,
            system_prompt,
            model_handle,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use tardis_gallifrey::domain::Entity;

    #[test]
    fn test_summon_entity() {
        let gallifrey = Arc::new(Gallifrey::new());

        // Insert a historical entity
        let mut props = HashMap::new();
        props.insert("status".to_string(), serde_json::json!("deprecated"));

        let past_time = Utc::now() - chrono::Duration::days(365);
        let active_interval = BiTemporalInterval::with_valid_time(
            tardis_common::temporal::TimeRange::starting_at(past_time)
        );

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Service".to_string(),
            name: "LegacyAPI".to_string(),
            properties: props,
            embedding: None,
            temporal: active_interval,
            source: None,
        };
        gallifrey.knowledge().insert_entity(entity).unwrap();

        let medium = Medium::new(gallifrey);
        let model = ModelHandle::new(1); // Fake handle

        // Try to summon it at a valid time
        let summon_time = Utc::now();
        let session = medium.summon("LegacyAPI", summon_time, model).unwrap();

        assert_eq!(session.entity_name, "LegacyAPI");
        assert!(session.system_prompt.contains("You are LegacyAPI"));
        assert!(session.system_prompt.contains("deprecated"));
    }

    #[test]
    fn test_summon_not_found() {
        let gallifrey = Arc::new(Gallifrey::new());
        let medium = Medium::new(gallifrey);
        let model = ModelHandle::new(1);

        let result = medium.summon("NonExistent", Utc::now(), model);
        assert!(result.is_err());
    }
}
