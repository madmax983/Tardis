//! The Temporal Fugue: An Alternative Timeline Simulator.
//!
//! "What if the database crashed yesterday?"
//!
//! This module allows exploring counterfactual scenarios by branching from a specific point
//! in the past and simulating the consequences using Vortex inference.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use chrono::{DateTime, Duration, Utc};
use std::fmt::Write;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::sync::Arc;
use tardis_common::id::ModelHandle;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::Gallifrey;
use tardis_vortex::{InferenceParams, Vortex};
use tracing::{info, instrument};

/// An event in the alternative timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FugueEvent {
    /// Relative time offset from the divergence point (e.g., "+2h", "Immediate").
    pub time_offset: String,
    /// Description of the event.
    pub description: String,
}

/// The result of a Fugue simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FugueResult {
    /// A narrative description of the alternative timeline.
    pub narrative: String,
    /// A list of specific divergent events.
    pub events: Vec<FugueEvent>,
}

/// The Fugue engine.
#[derive(Debug)]
pub struct Fugue {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    paper: PsychicPaper,
}

impl Fugue {
    /// Create a new Fugue engine.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            vortex,
            gallifrey,
            paper: PsychicPaper::new(),
        }
    }

    /// Simulate an alternative timeline starting from a divergence point.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails or the output cannot be parsed.
    #[instrument(skip(self))]
    pub async fn simulate(
        &self,
        divergence_time: DateTime<Utc>,
        counterfactual: &str,
        horizon: Duration,
        model: ModelHandle,
    ) -> ChronosResult<FugueResult> {
        info!("Initiating Temporal Fugue at {}", divergence_time);

        // 1. Embed the counterfactual to find relevant context
        let embedding = self
            .vortex
            .embed(model, counterfactual)
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // 2. Scan history for relevant entities active at the divergence time
        let candidates_cell = std::sync::Mutex::new(Vec::new());

        self.gallifrey.knowledge().scan_history(|history| {
            if let Some(entity) = history
                .iter()
                .find(|e| e.temporal.active_at(divergence_time, divergence_time))
            {
                if let Some(emb) = &entity.embedding {
                    let score = cosine_similarity(&embedding, emb);
                    if score > 0.4 { // Lower threshold for prototype
                        if let Ok(mut c) = candidates_cell.lock() {
                            c.push((entity.clone(), score));
                        }
                    }
                }
            }
        }).map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        let mut candidates = candidates_cell.into_inner().unwrap_or_default();

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        // Take top 5
        let context_entities: Vec<Entity> = candidates.into_iter().take(5).map(|(e, _)| e).collect();

        // 3. Construct Prompt
        let horizon_desc = format!("{} hours", horizon.num_seconds() / 3600);

        let mut context_str = String::new();
        for entity in &context_entities {
            let _ = writeln!(
                &mut context_str,
                "- {} ({}): {}",
                entity.name,
                entity.entity_type,
                serde_json::to_string(&entity.properties).unwrap_or_default()
            );
        }

        if context_str.is_empty() {
            context_str = "No specific relevant system state found.".to_string();
        }

        let prompt = format!(
            "SYSTEM STATE at {divergence_time}:\n{context_str}\n\nHYPOTHETICAL SCENARIO: {counterfactual}\n\nTASK: Simulate the consequences of this scenario over the next {horizon_desc}. \
            Return a JSON object with two fields:\n\
            1. 'narrative': A short paragraph summarizing the timeline.\n\
            2. 'events': A list of objects, each having 'time_offset' (string) and 'description' (string).\n\
            Output ONLY JSON."
        );

        // 4. Inference
        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 1024,
                    temperature: 0.7,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // 5. Parse
        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let narrative = parsed
            .get("narrative")
            .and_then(Value::as_str)
            .unwrap_or("Simulation failed to generate narrative.")
            .to_string();

        let mut events = Vec::new();
        if let Some(arr) = parsed.get("events").and_then(Value::as_array) {
            for item in arr {
                let time_offset = item.get("time_offset").and_then(Value::as_str).unwrap_or("?").to_string();
                let description = item.get("description").and_then(Value::as_str).unwrap_or("Unknown").to_string();
                events.push(FugueEvent { time_offset, description });
            }
        }

        Ok(FugueResult { narrative, events })
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    #[tokio::test]
    async fn test_fugue_simulation() {
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Embedding
        vortex.set_mock_embedding(Box::new(|_, _| {
            Ok(vec![1.0, 0.0, 0.0]) // Unit vector X
        }));

        // Mock Inference
        vortex.set_mock_inference(Box::new(|_, _, _| {
            let response = json!({
                "narrative": "The system crashed but recovered automatically.",
                "events": [
                    {
                        "time_offset": "+5m",
                        "description": "Watchdog detected failure."
                    },
                    {
                        "time_offset": "+10m",
                        "description": "Service restarted."
                    }
                ]
            });
            Ok(response.to_string())
        }));

        let gallifrey = Arc::new(Gallifrey::new());

        // Insert a relevant entity
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "System".to_string(),
            name: "Database Cluster".to_string(),
            properties: HashMap::new(),
            embedding: Some(vec![0.9, 0.1, 0.0]), // Highly similar to [1,0,0]
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.knowledge().insert_entity(entity).unwrap();

        let fugue = Fugue::new(vortex, gallifrey);
        let model = ModelHandle::new(1);
        let divergence_time = Utc::now();

        let result = fugue
            .simulate(
                divergence_time,
                "Database crash",
                Duration::hours(1),
                model,
            )
            .await
            .unwrap();

        assert_eq!(result.narrative, "The system crashed but recovered automatically.");
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.events[0].description, "Watchdog detected failure.");
    }
}
