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
use tardis_vortex::{InferenceParams, ModelHandle, ModelLoadConfig, Vortex};

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
    pub const fn new(
        telemetry: Arc<TelemetryStore>,
        gallifrey: Arc<Gallifrey>,
        vortex: Option<Arc<Vortex>>,
    ) -> Self {
        Self {
            telemetry,
            gallifrey,
            vortex,
            model_handle: None,
        }
    }

    /// Set a specific model to use for diagnosis.
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
    #[allow(clippy::unused_async)]
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

        // Correlate with system changes
        let mut root_causes = Vec::new();
        if status != HealthStatus::Healthy {
            let now = Utc::now();
            let window = Duration::minutes(5);
            let from = now - window;

            if let Ok(changes) = self.gallifrey.system_state().get_changes(from, now) {
                for change in changes {
                    root_causes.push(format!(
                        "System change to {} at {} ({:?})",
                        change.path, change.timestamp, change.change_type
                    ));
                }
            }

            if root_causes.is_empty() {
                root_causes.push(
                    "No recent system changes found. Possible external factor or load spike."
                        .to_string(),
                );
            }
        }

        let mut prescription = (status != HealthStatus::Healthy).then(|| Prescription {
            description: "Check recent system state changes and error logs.".to_string(),
            auto_fix_command: None,
        });

        // Enhance with AI if available
        if status != HealthStatus::Healthy {
            if let Some(vortex) = &self.vortex {
                // Determine handle: use explicit if set, otherwise try to find/load one
                let target_handle = if let Some(h) = self.model_handle {
                    Some(h)
                } else if let Some(model_info) = vortex.list_models().first() {
                    vortex
                        .load_model(
                            model_info.path.to_str().unwrap_or(""),
                            ModelLoadConfig::default(),
                        )
                        .await
                        .ok()
                } else {
                    None
                };

                if let Some(handle) = target_handle {
                    let prompt = format!(
                        "System Diagnosis.\nSymptoms: {symptoms:?}\nRecent Changes: {root_causes:?}\n\nSuggest a root cause and a fix command."
                    );

                    if let Ok(response) = vortex
                        .infer(handle, &prompt, InferenceParams::default())
                        .await
                    {
                        // Simple heuristic parsing for now
                        prescription = Some(Prescription {
                            description: format!("AI Analysis: {response}"),
                            auto_fix_command: None,
                        });
                    }
                }
            }
        }

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
    async fn test_doctor_ai_diagnosis() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock inference
        vortex.set_mock_inference(Box::new(|_, _, _| {
            Ok("Root cause: flux capacitor overload".to_string())
        }));

        // Inject error to trigger diagnosis
        let error_event = EventData {
            span_id: None,
            trace_id: TraceId::generate(),
            timestamp: Utc::now(),
            level: Level::Error,
            message: "Flux instability".to_string(),
            fields: HashMap::new(),
            subsystem: Subsystem::Kernel,
        };
        telemetry.record_event(error_event).await.unwrap();

        let handle = ModelHandle::new(1);
        let doctor =
            SystemDoctor::new(Arc::clone(&telemetry), gallifrey, Some(vortex)).with_model(handle);

        let diagnosis = doctor.diagnose().await;

        assert_ne!(diagnosis.status, HealthStatus::Healthy);
        if let Some(prescription) = diagnosis.prescription {
            assert!(prescription.description.contains("flux capacitor"));
        } else {
            panic!("Expected AI prescription");
        }
    }
}
