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

/// Vital signs of the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    model_handle: Option<ModelHandle>,
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub const fn new(telemetry: Arc<TelemetryStore>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            telemetry,
            gallifrey,
            vortex: None,
            model_handle: None,
        }
    }

    /// Attach Vortex AI engine.
    #[must_use]
    pub fn with_vortex(mut self, vortex: Arc<Vortex>) -> Self {
        self.vortex = Some(vortex);
        self
    }

    /// Attach a model handle for AI inference.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_model(mut self, handle: ModelHandle) -> Self {
        self.model_handle = Some(handle);
        self
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
    #[allow(clippy::unused_async, clippy::too_many_lines)]
    pub async fn diagnose(&self) -> Diagnosis {
        let vitals = self.check_vitals();

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

        // Gather system changes
        let mut changes_desc = Vec::new();
        let now = Utc::now();
        let window = Duration::minutes(5);
        let from = now - window;

        if let Ok(changes) = self.gallifrey.system_state().get_changes(from, now) {
            for change in changes {
                changes_desc.push(format!(
                    "Change to {} at {} ({:?})",
                    change.path, change.timestamp, change.change_type
                ));
            }
        }

        // Correlate with system changes (Heuristic or AI)
        let mut root_causes = Vec::new();

        if let (Some(vortex), Some(handle)) = (&self.vortex, self.model_handle) {
            // AI-Enhanced Diagnosis
            if status != HealthStatus::Healthy {
                let changes_str = if changes_desc.is_empty() {
                    "None".to_string()
                } else {
                    changes_desc.join("\n- ")
                };

                let prompt = format!(
                    "System Doctor Diagnosis Request\n\
                     Time: {}\n\n\
                     Symptoms:\n- {}\n\n\
                     Recent Changes:\n- {}\n\n\
                     Analyze the correlation between the changes and symptoms.\n\
                     Identify the root cause and suggest a fix.",
                    now,
                    symptoms.join("\n- "),
                    changes_str
                );

                let params = InferenceParams {
                    max_tokens: 200,
                    temperature: 0.7,
                    ..InferenceParams::default()
                };

                match vortex.infer(handle, &prompt, params).await {
                    Ok(response) => {
                        root_causes.push(format!("[AI] {}", response.trim()));
                    }
                    Err(e) => {
                        root_causes.push(format!("[AI Error] Failed to consult Vortex: {e}"));
                    }
                }
            }
        } else {
            // Classic Heuristic Diagnosis
            if status != HealthStatus::Healthy {
                for desc in &changes_desc {
                    root_causes.push(format!("System {desc}"));
                }

                if root_causes.is_empty() {
                    root_causes.push(
                        "No recent system changes found. Possible external factor or load spike."
                            .to_string(),
                    );
                }
            }
        }

        let prescription = (status != HealthStatus::Healthy).then(|| Prescription {
            description: if !root_causes.is_empty() && root_causes[0].contains("[AI]") {
                "Follow AI recommendations above.".to_string()
            } else {
                "Check recent system state changes and error logs.".to_string()
            },
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
    async fn test_doctor_diagnosis() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey);

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
    async fn test_doctor_ai_diagnosis() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock Vortex Inference
        let mock_response = "The high latency is likely caused by the kernel explosion.";
        vortex.set_mock_inference(Box::new(move |_handle, _prompt, _params| {
            Ok(mock_response.to_string())
        }));

        let handle = ModelHandle::new(1); // Dummy handle

        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey)
            .with_vortex(vortex)
            .with_model(handle);

        // Inject an error to trigger diagnosis
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

        // Diagnose
        let diagnosis = doctor.diagnose().await;

        // Verify AI usage
        assert_ne!(diagnosis.status, HealthStatus::Healthy);
        assert!(!diagnosis.root_causes.is_empty());
        assert!(diagnosis.root_causes[0].contains("[AI]"));
        assert!(diagnosis.root_causes[0].contains("kernel explosion"));
    }
}
