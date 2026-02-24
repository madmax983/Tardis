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

const DIAGNOSIS_WINDOW_MINUTES: i64 = 5;
const CRITICAL_ERROR_THRESHOLD: usize = 10;
const HIGH_CONCURRENCY_THRESHOLD: usize = 100;
const HIGH_LATENCY_THRESHOLD_MS: u64 = 500;

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
        let window = Duration::minutes(DIAGNOSIS_WINDOW_MINUTES);
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
            .fold((0u64, 0u64), |(sum, count), dur| {
                (sum.saturating_add(dur), count + 1)
            });

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

        Self::check_error_symptoms(&vitals, &mut symptoms, &mut status);
        Self::check_concurrency_symptoms(&vitals, &mut symptoms, &mut status);
        Self::check_latency_symptoms(&vitals, &mut symptoms, &mut status);

        // Gather system changes
        let now = Utc::now();
        let changes_desc = self.get_recent_changes(now);

        // Correlate with system changes (Heuristic or AI)
        let root_causes = self
            .determine_root_causes(status, &symptoms, &changes_desc, now)
            .await;

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

    fn check_error_symptoms(
        vitals: &Vitals,
        symptoms: &mut Vec<String>,
        status: &mut HealthStatus,
    ) {
        if vitals.error_count > 0 {
            *status = if vitals.error_count > CRITICAL_ERROR_THRESHOLD {
                HealthStatus::Critical
            } else {
                HealthStatus::Degraded
            };
            symptoms.push(format!(
                "Found {} errors in last {}m",
                vitals.error_count, DIAGNOSIS_WINDOW_MINUTES
            ));
        }
    }

    fn check_concurrency_symptoms(
        vitals: &Vitals,
        symptoms: &mut Vec<String>,
        status: &mut HealthStatus,
    ) {
        if vitals.active_spans > HIGH_CONCURRENCY_THRESHOLD {
            symptoms.push(format!(
                "High concurrency: {} active spans",
                vitals.active_spans
            ));
            if *status == HealthStatus::Healthy {
                *status = HealthStatus::Degraded;
            }
        }
    }

    fn check_latency_symptoms(
        vitals: &Vitals,
        symptoms: &mut Vec<String>,
        status: &mut HealthStatus,
    ) {
        if vitals.average_latency_ms > HIGH_LATENCY_THRESHOLD_MS {
            symptoms.push(format!(
                "High latency: {}ms average",
                vitals.average_latency_ms
            ));
            if *status == HealthStatus::Healthy {
                *status = HealthStatus::Degraded;
            }
        }
    }

    fn get_recent_changes(&self, now: chrono::DateTime<Utc>) -> Vec<String> {
        let mut changes_desc = Vec::new();
        let window = Duration::minutes(DIAGNOSIS_WINDOW_MINUTES);
        let from = now - window;

        if let Ok(changes) = self.gallifrey.system_state().get_changes(from, now) {
            for change in changes {
                changes_desc.push(format!(
                    "Change to {} at {} ({:?})",
                    change.path, change.timestamp, change.change_type
                ));
            }
        }
        changes_desc
    }

    async fn determine_root_causes(
        &self,
        status: HealthStatus,
        symptoms: &[String],
        changes_desc: &[String],
        now: chrono::DateTime<Utc>,
    ) -> Vec<String> {
        let mut root_causes = Vec::new();
        if let (Some(vortex), Some(handle)) = (&self.vortex, self.model_handle) {
            Self::consult_vortex(
                vortex,
                handle,
                status,
                symptoms,
                changes_desc,
                now,
                &mut root_causes,
            )
            .await;
        } else {
            Self::consult_heuristics(status, changes_desc, &mut root_causes);
        }
        root_causes
    }

    async fn consult_vortex(
        vortex: &Vortex,
        handle: ModelHandle,
        status: HealthStatus,
        symptoms: &[String],
        changes_desc: &[String],
        now: chrono::DateTime<Utc>,
        root_causes: &mut Vec<String>,
    ) {
        if status == HealthStatus::Healthy {
            return;
        }

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

    fn consult_heuristics(
        status: HealthStatus,
        changes_desc: &[String],
        root_causes: &mut Vec<String>,
    ) {
        if status == HealthStatus::Healthy {
            return;
        }

        for desc in changes_desc {
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
            start_instant: Instant::now()
                .checked_sub(std::time::Duration::from_millis(1000))
                .unwrap_or_else(Instant::now), // Started 1s ago
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

    #[tokio::test]
    async fn test_check_vitals_safe() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey);

        // Add multiple spans to verify summation logic
        for _ in 0..10 {
            let span = SpanData {
                trace_id: TraceId::generate(),
                span_id: SpanId::generate(),
                parent_id: None,
                name: "op".to_string(),
                subsystem: Subsystem::Vortex,
                start_instant: Instant::now()
                    .checked_sub(std::time::Duration::from_millis(100))
                    .unwrap_or_else(Instant::now),
                start_time: Utc::now() - Duration::milliseconds(100),
                end_time: Some(Utc::now()),
                attributes: HashMap::new(),
                level: Level::Info,
            };
            telemetry.record_span(span).await.unwrap();
        }

        let vitals = doctor.check_vitals();
        assert_eq!(vitals.active_spans, 0); // They are completed
                                            // 10 spans of ~100ms = 1000ms total. 1000ms / 10 = 100ms average.
                                            // Allowing some variance due to execution time.
        assert!(
            vitals.average_latency_ms >= 90 && vitals.average_latency_ms <= 200,
            "Average latency {} should be around 100ms",
            vitals.average_latency_ms
        );
    }
}
