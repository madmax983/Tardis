//! The System Doctor 🩺
//!
//! A self-diagnostic tool that correlates telemetry data (symptoms)
//! with system state changes (causes) to prescribe fixes.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::Level;
use tardis_vortex::{InferenceParams, ModelHandle, Vortex};
use tracing::{error, info};

/// Vital signs of the system.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vitals {
    /// Number of errors in the last window.
    pub error_count: usize,
    /// Number of active spans.
    pub active_spans: usize,
    /// Average latency (mocked for now).
    pub average_latency_ms: u64,
}

/// A diagnosis of the system health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Health status.
    pub status: HealthStatus,
    /// Observed symptoms.
    pub symptoms: Vec<String>,
    /// Potential root causes.
    pub root_causes: Vec<String>,
    /// Recommended actions.
    pub prescription: Option<Prescription>,
}

/// System health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All systems nominal.
    Healthy,
    /// Minor issues detected.
    Degraded,
    /// Critical failure imminent.
    Critical,
}

/// A recommended course of action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prescription {
    /// Description of the fix.
    pub description: String,
    /// Command to run (if any).
    pub auto_fix_command: Option<String>,
}

/// The System Doctor.
#[derive(Debug)]
pub struct SystemDoctor {
    telemetry: Arc<TelemetryStore>,
    gallifrey: Arc<Gallifrey>,
    vortex: Option<Arc<Vortex>>,
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub const fn new(
        telemetry: Arc<TelemetryStore>,
        gallifrey: Arc<Gallifrey>,
        vortex: Option<Arc<Vortex>>,
    ) -> Self {
        Self {
            telemetry,
            gallifrey,
            vortex,
        }
    }

    /// Check system vitals.
    ///
    /// Analyzes the last 5 minutes of telemetry data.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn check_vitals(&self) -> Vitals {
        let now = Utc::now();
        let window = Duration::minutes(5);
        let from = now - window;

        // Count errors
        let events = self.telemetry.events_in_range(from, now);
        let error_count = events
            .iter()
            .filter(|e| e.data.level == Level::Error)
            .count();

        // Check active spans
        let active_spans = self.telemetry.spans_at(now, None).len();

        // Calculate average latency of completed spans
        let completed_spans = self.telemetry.spans_in_range(from, now);
        let (total_latency, count) = completed_spans
            .iter()
            .filter_map(|s| s.data.duration_ns())
            .fold((0, 0u64), |(sum, count), dur| (sum + dur, count + 1));

        let average_latency_ms = if count > 0 {
            (total_latency / count) / 1_000_000
        } else {
            0
        };

        Vitals {
            error_count,
            active_spans,
            average_latency_ms,
        }
    }

    /// Run a full diagnosis.
    #[allow(clippy::unused_async)]
    pub async fn diagnose(&self) -> Diagnosis {
        let vitals = self.check_vitals();

        // Gather context
        let now = Utc::now();
        let window = Duration::minutes(5);
        let from = now - window;

        let mut recent_changes = Vec::new();
        if let Ok(changes) = self.gallifrey.system_state().get_changes(from, now) {
            for change in changes {
                recent_changes.push(format!(
                    "System change to {} at {} ({:?})",
                    change.path, change.timestamp, change.change_type
                ));
            }
        }

        // Try AI Diagnosis first
        if let Some(vortex) = &self.vortex {
            let loaded_models = vortex.list_loaded_models();
            if let Some((handle, _)) = loaded_models.first() {
                info!("Running AI diagnosis with model {}", handle);
                if let Ok(diagnosis) = self
                    .diagnose_with_ai(vortex, *handle, &vitals, &recent_changes)
                    .await
                {
                    return diagnosis;
                }
                error!("AI Diagnosis failed, falling back to heuristic.");
            }
        }

        // Fallback to Heuristic Diagnosis
        self.diagnose_heuristic(vitals, recent_changes)
    }

    async fn diagnose_with_ai(
        &self,
        vortex: &Vortex,
        handle: ModelHandle,
        vitals: &Vitals,
        changes: &[String],
    ) -> anyhow::Result<Diagnosis> {
        let changes_text = if changes.is_empty() {
            "No recent changes.".to_string()
        } else {
            changes.join("\n")
        };

        let prompt = format!(
            "Analyze the following system status and identify the root cause of any issues.\n\
             \n\
             METRICS:\n\
             - Error Count: {}\n\
             - Active Spans: {}\n\
             - Average Latency: {} ms\n\
             \n\
             RECENT CHANGES:\n\
             {}\n\
             \n\
             Provide a diagnosis in the following JSON format ONLY:\n\
             {{\n\
               \"status\": \"Healthy\" | \"Degraded\" | \"Critical\",\n\
               \"symptoms\": [\"symptom1\", \"symptom2\"],\n\
               \"root_causes\": [\"cause1\", \"cause2\"],\n\
               \"prescription\": {{\n\
                 \"description\": \"fix description\",\n\
                 \"auto_fix_command\": \"optional command\"\n\
               }}\n\
             }}",
            vitals.error_count, vitals.active_spans, vitals.average_latency_ms, changes_text
        );

        let params = InferenceParams {
            max_tokens: 500,
            temperature: 0.1, // Low temp for deterministic JSON
            ..InferenceParams::default()
        };

        let response = vortex.infer(handle, &prompt, params).await?;

        // Try to parse JSON from the response
        // Find the first '{' and last '}'
        let start = response.find('{').unwrap_or(0);
        let end = response.rfind('}').map_or(response.len(), |i| i + 1);
        let json_str = &response[start..end];

        let diagnosis: Diagnosis = serde_json::from_str(json_str)?;

        Ok(diagnosis)
    }

    #[allow(clippy::unused_self)]
    fn diagnose_heuristic(&self, vitals: Vitals, changes: Vec<String>) -> Diagnosis {
        let mut status = HealthStatus::Healthy;
        let mut symptoms = Vec::new();

        if vitals.error_count > 0 {
            status = if vitals.error_count > 10 {
                HealthStatus::Critical
            } else {
                HealthStatus::Degraded
            };
            symptoms.push(format!("Found {} errors in last 5m", vitals.error_count));
        }

        if vitals.active_spans > 100 {
            symptoms.push(format!(
                "High concurrency: {} active spans",
                vitals.active_spans
            ));
            if status == HealthStatus::Healthy {
                status = HealthStatus::Degraded;
            }
        }

        if vitals.average_latency_ms > 500 {
            symptoms.push(format!(
                "High latency: {}ms average",
                vitals.average_latency_ms
            ));
            if status == HealthStatus::Healthy {
                status = HealthStatus::Degraded;
            }
        }

        let mut root_causes = changes;
        if status != HealthStatus::Healthy && root_causes.is_empty() {
            root_causes.push(
                "No recent system changes found. Possible external factor or load spike."
                    .to_string(),
            );
        }

        let prescription = (status != HealthStatus::Healthy).then(|| Prescription {
            description: "Check recent system state changes and error logs.".to_string(),
            auto_fix_command: None,
        });

        Diagnosis {
            status,
            symptoms,
            root_causes,
            prescription,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Instant;
    use tardis_telemetry::types::{SpanId, Subsystem, TraceId};
    use tardis_telemetry::userspace::layer::{EventData, SpanData};

    #[tokio::test]
    async fn test_doctor_diagnosis_heuristic() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        // No Vortex
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey, None);

        // Healthy check
        let diagnosis = doctor.diagnose().await;
        assert_eq!(diagnosis.status, HealthStatus::Healthy);

        // Inject an error
        let error_event = EventData {
            span_id: None,
            trace_id: TraceId::generate(),
            timestamp: Utc::now(),
            level: Level::Error,
            message: "Something exploded".to_string(),
            fields: HashMap::new(),
            subsystem: Subsystem::Kernel,
        };
        telemetry.record_event(error_event).await.unwrap();

        // Inject high latency span
        let slow_span = SpanData {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_id: None,
            name: "slow_op".to_string(),
            subsystem: Subsystem::Vortex,
            start_instant: Instant::now() - std::time::Duration::from_millis(1000), // Started 1s ago
            start_time: Utc::now() - Duration::milliseconds(1000),
            end_time: Some(Utc::now()), // Just finished, duration 1s
            attributes: HashMap::new(),
            level: Level::Info,
        };
        telemetry.record_span(slow_span).await.unwrap();

        // Diagnose again
        let diagnosis = doctor.diagnose().await;
        assert_ne!(diagnosis.status, HealthStatus::Healthy);
        assert!(diagnosis.symptoms.iter().any(|s| s.contains("errors")));
        assert!(diagnosis
            .symptoms
            .iter()
            .any(|s| s.contains("High latency")));
    }

    #[tokio::test]
    async fn test_doctor_diagnosis_ai() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock a loaded model using manual registration
        let _handle = vortex.register_mock_model("mock-model").unwrap();

        // Mock inference response
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok(r#"{
                "status": "Critical",
                "symptoms": ["AI detected error"],
                "root_causes": ["Solar flare"],
                "prescription": {
                    "description": "Rotate shield frequencies",
                    "auto_fix_command": "shields rotate"
                }
            }"#.to_string())
        }));

        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey, Some(vortex));

        // Diagnosis should use AI
        let diagnosis = doctor.diagnose().await;

        assert_eq!(diagnosis.status, HealthStatus::Critical);
        assert_eq!(diagnosis.root_causes[0], "Solar flare");
        assert_eq!(diagnosis.prescription.unwrap().auto_fix_command.unwrap(), "shields rotate");
    }
}
