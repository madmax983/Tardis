//! Prophecy: The Future Forecast Engine.
//!
//! "History is a burden. Stories can make us fly."
//!
//! This module utilizes the Vortex LLM to predict likely future system states
//! based on current context.

use crate::error::{ChronosError, ChronosResult};
use crate::experimental::psychic_paper::{Intent, PsychicPaper};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tardis_common::domain::Entity;
use tardis_common::id::{EntityId, ModelHandle};
use tardis_common::llm::InferenceParams;
use tardis_common::temporal::{BiTemporalInterval, TimeRange};
use tardis_common::traits::{GallifreyService, VortexService};
use tracing::{info, instrument};

/// Predicted system metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedMetrics {
    /// The process name.
    pub process_name: String,
    /// Predicted CPU usage percentage.
    pub cpu_percent: f32,
    /// Predicted memory usage in bytes.
    pub memory_bytes: u64,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
}

/// The Prophet engine.
#[derive(Debug)]
pub struct Prophet {
    vortex: Arc<dyn VortexService>,
    gallifrey: Arc<dyn GallifreyService>,
    paper: PsychicPaper,
}

impl Prophet {
    /// Create a new Prophet.
    #[must_use]
    pub fn new(vortex: Arc<dyn VortexService>, gallifrey: Arc<dyn GallifreyService>) -> Self {
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
            .await
            .map_err(ChronosError::Common)?;

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

    /// Forecast future metrics for a specific process based on history.
    ///
    /// # Errors
    ///
    /// Returns an error if history cannot be retrieved or inference fails.
    #[instrument(skip(self))]
    pub async fn forecast_metrics(
        &self,
        process_name: &str,
        horizon: Duration,
        model: ModelHandle,
    ) -> ChronosResult<PredictedMetrics> {
        info!("Forecasting metrics for process: {}", process_name);

        // 1. Get History
        let history = self
            .gallifrey
            .get_snapshot_history(10)
            .await
            .map_err(ChronosError::Common)?;

        // 2. Extract Metrics for the process
        // We take the last 10 snapshots to form a trend
        let series: Vec<_> = history
            .iter()
            .rev()
            .take(10)
            .filter_map(|snapshot| {
                // Find process by name
                snapshot
                    .state
                    .processes
                    .values()
                    .find(|p| p.name == process_name)
                    .map(|p| {
                        json!({
                            "time": snapshot.timestamp.to_rfc3339(),
                            "cpu": p.cpu_percent,
                            "mem": p.memory_bytes
                        })
                    })
            })
            .collect();

        if series.is_empty() {
            return Err(ChronosError::Common(tardis_common::Error::Internal(
                format!("No history found for process: {process_name}"),
            )));
        }

        // Reverse back to chronological order
        let series: Vec<_> = series.into_iter().rev().collect();
        let series_json = serde_json::to_string_pretty(&series).unwrap_or_default();

        let horizon_minutes = horizon.as_secs() / 60;

        // 3. Construct Prompt
        let prompt = format!(
            "You are a predictive system analyst. Analyze the following metric history for process '{process_name}'.\n\
             History:\n{series_json}\n\n\
             Predict the CPU percentage and Memory usage (in bytes) for {horizon_minutes} minutes into the future.\n\
             Return ONLY a JSON object with keys: 'cpu_percent' (float), 'memory_bytes' (int), and 'confidence' (float 0.0-1.0).\n\
             Do not include markdown blocks or explanation."
        );

        // 4. Infer
        let response = self
            .vortex
            .infer(
                model,
                &prompt,
                InferenceParams {
                    max_tokens: 128,
                    temperature: 0.3, // Lower temperature for numbers
                    ..Default::default()
                },
            )
            .await
            .map_err(ChronosError::Common)?;

        // 5. Parse
        let parsed = self
            .paper
            .interpret(&response, Intent::Json)
            .map_err(|e| ChronosError::Common(tardis_common::Error::Internal(e)))?;

        let cpu_percent = parsed
            .get("cpu_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let memory_bytes = parsed
            .get("memory_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let confidence = parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        Ok(PredictedMetrics {
            process_name: process_name.to_string(),
            cpu_percent,
            memory_bytes,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use tardis_common::domain::{Change, Message, ProcessState, Snapshot, SnapshotTrigger, SystemState};
    use tardis_common::id::{SessionId, SnapshotId};
    use tardis_common::llm::ModelLoadConfig;
    use tardis_common::temporal::TemporalQuery;
    use tardis_common::traits::QueryResult;
    use tardis_common::Result;

    #[derive(Debug)]
    struct MockVortex;

    #[async_trait]
    impl VortexService for MockVortex {
        async fn load_model(&self, _path: &str, _config: ModelLoadConfig) -> Result<ModelHandle> {
            Ok(ModelHandle::new(1))
        }
        async fn unload_model(&self, _handle: ModelHandle) -> Result<()> {
            Ok(())
        }
        async fn infer(
            &self,
            _handle: ModelHandle,
            prompt: &str,
            _params: InferenceParams,
        ) -> Result<String> {
            if prompt.contains("Predict the CPU percentage") {
                return Ok(json!({
                    "cpu_percent": 75.5,
                    "memory_bytes": 1024000,
                    "confidence": 0.95
                }).to_string());
            }

            // Return a simulated prediction for foresee
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
        }
        async fn embed(&self, _handle: ModelHandle, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![])
        }
    }

    #[derive(Debug)]
    struct MockGallifrey;

    #[async_trait]
    impl GallifreyService for MockGallifrey {
        async fn insert(&self, _entity: Entity) -> Result<EntityId> {
            Ok(EntityId::new())
        }
        async fn get_history(&self, _id: EntityId) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn search_knowledge(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<Entity>> {
            Ok(vec![])
        }
        async fn query(&self, _query: &str, _temporal: TemporalQuery) -> Result<QueryResult> {
            Ok(QueryResult {
                nodes: vec![],
                execution_time_ms: 0,
                truncated: false,
            })
        }
        async fn get_recent_messages(
            &self,
            _session_id: SessionId,
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn search_conversation(
            &self,
            _embedding: &[f32],
            _limit: usize,
        ) -> Result<Vec<Message>> {
            Ok(vec![])
        }
        async fn find_snapshot(&self, _timestamp: chrono::DateTime<chrono::Utc>) -> Result<Option<Snapshot>> {
            Ok(None)
        }
        async fn get_snapshot_history(&self, _limit: usize) -> Result<Vec<Snapshot>> {
             let mut snapshots = Vec::new();
             // Create a dummy snapshot with a process
             let process = ProcessState {
                 pid: 1234,
                 name: "test_process".to_string(),
                 status: "running".to_string(),
                 memory_bytes: 500000,
                 cpu_percent: 10.0,
             };
             let mut processes = HashMap::new();
             processes.insert(1234, process);

             let state = SystemState {
                 processes,
                 config: HashMap::new(),
                 files: HashMap::new(),
             };

             snapshots.push(Snapshot {
                 id: SnapshotId::new(),
                 name: "snap1".to_string(),
                 timestamp: chrono::Utc::now(),
                 trigger: SnapshotTrigger::Scheduled,
                 state,
                 checksum: "abc".to_string(),
             });
             Ok(snapshots)
        }
        async fn record_change(&self, _change: Change) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_foresee() {
        let vortex = Arc::new(MockVortex);
        let gallifrey = Arc::new(MockGallifrey);
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
    }

    #[tokio::test]
    async fn test_forecast_metrics() {
        let vortex = Arc::new(MockVortex);
        let gallifrey = Arc::new(MockGallifrey);
        let prophet = Prophet::new(vortex, gallifrey);
        let model = ModelHandle::new(1);

        let metrics = prophet
            .forecast_metrics(
                "test_process",
                Duration::from_secs(300),
                model,
            )
            .await
            .unwrap();

        assert_eq!(metrics.process_name, "test_process");
        assert!((metrics.cpu_percent - 75.5).abs() < 0.001);
        assert_eq!(metrics.memory_bytes, 1024000);
    }
}
