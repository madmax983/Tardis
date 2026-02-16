//! Prophecy: The Future Forecast Engine.
//!
//! "History is a burden. Stories can make us fly."
//!
//! This module utilizes the Vortex LLM to predict likely future system states
//! based on current context.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use std::sync::Arc;
use std::time::Duration;
use tardis_common::id::{EntityId, ModelHandle};
use tardis_common::temporal::{BiTemporalInterval, TimeRange};
use tardis_gallifrey::domain::Entity;
use tardis_vortex::InferenceParams;
use tardis_vortex::VortexService;
use tracing::{info, instrument};

/// The Prophet engine.
#[derive(Debug)]
pub struct Prophet {
    vortex: Arc<dyn VortexService>,
    paper: PsychicPaper,
}

impl Prophet {
    /// Create a new Prophet.
    #[must_use]
    pub fn new(vortex: Arc<dyn VortexService>) -> Self {
        Self {
            vortex,
            paper: PsychicPaper::new(),
        }
    }

    /// Predict future events based on context.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails or the output cannot be parsed.
    #[instrument(skip(self))]
    pub async fn foresee(
        &self,
        context: &str,
        horizon: Duration,
        model: ModelHandle,
    ) -> ChronosResult<Vec<Entity>> {
        let horizon_minutes = horizon.as_secs() / 60;
        info!(
            "Consulting the Prophet: predicting {} minutes into the future...",
            horizon_minutes
        );

        let prompt = format!(
            "Context: {context}\n\nBased on the above context, predict 3 likely future system events that will occur in the next {horizon_minutes} minutes. Return the result as a JSON list of objects. Each object must have 'name' (string), 'type' (string), and 'description' (string) fields. Do not include any explanation, just the JSON."
        );

        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 512,
                    temperature: 0.7,
                    ..Default::default()
                },
            )
            .await?;

        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let mut predictions = Vec::new();

        if let Some(array) = parsed.as_array() {
            for item in array {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Prediction")
                    .to_string();
                let entity_type = item
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Prediction")
                    .to_string();

                // Construct properties from the item
                let mut properties = std::collections::HashMap::new();
                if let Some(obj) = item.as_object() {
                    for (k, v) in obj {
                        properties.insert(k.clone(), v.clone());
                    }
                }

                // Create the future entity
                // Valid time starts in the future (now + horizon)
                // Transaction time is now
                let now = chrono::Utc::now();
                let future_time = now
                    + chrono::Duration::from_std(horizon).unwrap_or(chrono::Duration::seconds(0));

                let valid_time = TimeRange::starting_at(future_time);
                let temporal = BiTemporalInterval::with_valid_time(valid_time);

                // Assume prediction is valid for a short window, say 1 hour, or open ended?
                // Let's make it open ended for now, or maybe same as horizon?
                // Let's just set valid_time start.

                let entity = Entity {
                    id: EntityId::new(),
                    entity_type,
                    name,
                    properties,
                    embedding: None,
                    temporal,
                    source: Some("Chronos::Prophecy".to_string()),
                };

                predictions.push(entity);
            }
        }

        info!("Prophet foresaw {} events.", predictions.len());
        Ok(predictions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_foresee() {
        let vortex = Arc::new(Vortex::new().unwrap());
        vortex.set_mock_inference(Box::new(|_, _, _| {
            let response = json!([
                {
                    "name": "High CPU Alert",
                    "type": "Alert",
                    "description": "CPU usage will exceed 90% due to backup process."
                },
                {
                    "name": "Disk Full Warning",
                    "type": "Warning",
                    "description": "Log rotation will fail due to disk space."
                }
            ]);
            Ok(response.to_string())
        }));

        let prophet = Prophet::new(vortex);
        let model = ModelHandle::new(1);

        let predictions = prophet
            .foresee(
                "System load is increasing.",
                Duration::from_secs(300),
                model,
            )
            .await
            .unwrap();

        assert_eq!(predictions.len(), 2);

        let first = &predictions[0];
        assert_eq!(first.name, "High CPU Alert");
        assert_eq!(first.entity_type, "Alert");

        // Verify time travel logic
        let now = chrono::Utc::now();
        // The valid time should be in the future (approx 5 mins from now)
        // We allow some delta for execution time
        let expected_future = now + chrono::Duration::minutes(5);
        let valid_time = first.temporal.valid_time.start;

        let diff = valid_time
            .signed_duration_since(expected_future)
            .num_seconds()
            .abs();
        assert!(
            diff < 5,
            "Prediction time should be ~5 minutes in the future"
        );
    }
}
