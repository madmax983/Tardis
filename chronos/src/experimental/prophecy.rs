//! Prophecy: The Future Forecast Engine.
//!
//! "History is a burden. Stories can make us fly."
//!
//! This module utilizes the Vortex LLM to predict likely future system states
//! based on current context.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tardis_common::domain::Entity;
use tardis_common::id::{EntityId, ModelHandle};
use tardis_common::llm::InferenceParams;
use tardis_common::temporal::{BiTemporalInterval, TimeRange};
use tardis_gallifrey::Gallifrey;
use tardis_vortex::Vortex;
use tracing::{info, instrument};

/// Predicted system metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedMetrics {
    /// Time of prediction.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Predicted CPU usage (percent).
    pub cpu_percent: f32,
    /// Predicted memory usage (bytes).
    pub memory_bytes: u64,
}

/// The Prophet engine.
#[derive(Debug)]
pub struct Prophet {
    vortex: Arc<Vortex>,
    gallifrey: Arc<Gallifrey>,
    paper: PsychicPaper,
}

impl Prophet {
    /// Create a new Prophet.
    #[must_use]
    pub const fn new(vortex: Arc<Vortex>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            vortex,
            gallifrey,
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

    /// Forecast future system metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if history cannot be retrieved or inference fails.
    #[instrument(skip(self))]
    pub async fn forecast_metrics(
        &self,
        minutes: u32,
        model: ModelHandle,
    ) -> ChronosResult<Vec<PredictedMetrics>> {
        info!("Forecasting metrics for the next {} minutes", minutes);

        // 1. Fetch history
        let snapshots = self
            .gallifrey
            .system_state()
            .list_snapshots()
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // Take last 20 snapshots
        let recent: Vec<_> = snapshots.iter().rev().take(20).rev().collect();

        if recent.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Build time series prompt
        let series: Vec<_> = recent
            .iter()
            .map(|s| {
                // Aggregate CPU/Memory from all processes
                let total_cpu: f32 = s.state.processes.values().map(|p| p.cpu_percent).sum();
                let total_mem: u64 = s.state.processes.values().map(|p| p.memory_bytes).sum();

                serde_json::json!({
                    "timestamp": s.timestamp,
                    "cpu": total_cpu,
                    "memory": total_mem
                })
            })
            .collect();

        let prompt = format!(
            "History:\n{}\n\nPredict the next {} data points (1 minute interval). Return strictly a JSON list of objects with 'timestamp', 'cpu', 'memory'.",
            serde_json::to_string_pretty(&series).unwrap_or_default(),
            minutes
        );

        // 3. Infer
        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 1024,
                    temperature: 0.2, // Lower temp for logic/numbers
                    ..Default::default()
                },
            )
            .await?;

        // 4. Parse
        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let mut metrics = Vec::new();
        if let Some(array) = parsed.as_array() {
            for item in array {
                #[allow(clippy::cast_possible_truncation)]
                let cpu = item
                    .get("cpu")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0) as f32;
                let memory = item
                    .get("memory")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);

                // Handle timestamp parsing
                let timestamp = if let Some(ts_str) =
                    item.get("timestamp").and_then(serde_json::Value::as_str)
                {
                    chrono::DateTime::parse_from_rfc3339(ts_str)
                        .map_or_else(|_| chrono::Utc::now(), |dt| dt.with_timezone(&chrono::Utc))
                } else {
                    chrono::Utc::now()
                };

                metrics.push(PredictedMetrics {
                    timestamp,
                    cpu_percent: cpu,
                    memory_bytes: memory,
                });
            }
        }

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_foresee() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());
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

        let prophet = Prophet::new(vortex, gallifrey);
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

    #[tokio::test]
    async fn test_forecast_metrics() {
        let vortex = Arc::new(Vortex::new().unwrap());
        let gallifrey = Arc::new(Gallifrey::new());

        // Mock Vortex response
        vortex.set_mock_inference(Box::new(|_, _, _| {
            // Generate some dummy predictions
            let predictions = vec![
                json!({
                    "timestamp": "2024-01-01T12:01:00Z",
                    "cpu": 50.5,
                    "memory": 102400
                }),
                json!({
                    "timestamp": "2024-01-01T12:02:00Z",
                    "cpu": 60.0,
                    "memory": 204800
                }),
            ];
            Ok(json!(predictions).to_string())
        }));

        // Populate Gallifrey with some data
        let state = tardis_common::domain::SystemState {
            processes: std::collections::HashMap::new(),
            config: std::collections::HashMap::new(),
            files: std::collections::HashMap::new(),
        };

        gallifrey
            .system_state()
            .take_snapshot(
                "test_snapshot",
                tardis_common::domain::SnapshotTrigger::Manual,
                state,
            )
            .unwrap();

        let prophet = Prophet::new(vortex, gallifrey);
        let model = ModelHandle::new(1);

        let predictions = prophet.forecast_metrics(2, model).await.unwrap();

        assert_eq!(predictions.len(), 2);
        assert!((predictions[0].cpu_percent - 50.5).abs() < f32::EPSILON);
        assert_eq!(predictions[1].memory_bytes, 204800);
    }
}
