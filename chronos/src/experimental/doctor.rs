//! The System Doctor 🩺
//!
//! A self-diagnostic tool that correlates telemetry data (symptoms)
//! with system state changes (causes) to prescribe fixes.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tardis_gallifrey::Gallifrey;
use tardis_gallifrey::SystemEntropy;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::Level;
use chrono::{Duration, Utc};
use std::fmt::Write;

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

impl Default for Vitals {
    fn default() -> Self {
        Self {
            error_count: 0,
            active_spans: 0,
            average_latency_ms: 0,
        }
    }
}

/// A diagnosis of the system health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Health status.
    pub status: HealthStatus,
    /// Observed symptoms.
    pub symptoms: Vec<String>,
    /// System entropy metrics.
    pub entropy: Option<SystemEntropy>,
    /// Temporal heatmap (ASCII representation).
    pub heatmap: Option<String>,
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
    telemetry: Option<Arc<TelemetryStore>>,
    gallifrey: Arc<Gallifrey>,
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub fn new(telemetry: Option<Arc<TelemetryStore>>, gallifrey: Arc<Gallifrey>) -> Self {
        Self {
            telemetry,
            gallifrey,
        }
    }

    /// Check system vitals.
    ///
    /// Analyzes the last 5 minutes of telemetry data.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn check_vitals(&self) -> Vitals {
        let store = match &self.telemetry {
            Some(s) => s,
            None => return Vitals::default(),
        };

        let now = Utc::now();
        let window = Duration::minutes(5);
        let from = now - window;

        // Count errors
        let events = store.events_in_range(from, now);
        let error_count = events
            .iter()
            .filter(|e| e.data.level == Level::Error)
            .count();

        // Check active spans
        let active_spans = store.spans_at(now, None).len();

        // Calculate average latency of completed spans
        let completed_spans = store.spans_in_range(from, now);
        let (total_latency, count) = completed_spans
            .iter()
            .filter_map(|s| s.data.duration_ns())
            .fold((0, 0), |(sum, count), dur| (sum + dur, count + 1));

        let average_latency_ms = if count > 0 {
            (total_latency / count as u64) / 1_000_000
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

        // --- Telemetry Checks ---
        if vitals.error_count > 0 {
            status = if vitals.error_count > 10 {
                HealthStatus::Critical
            } else {
                HealthStatus::Degraded
            };
            symptoms.push(format!("Found {} errors in last 5m", vitals.error_count));
        }

        if vitals.active_spans > 100 {
            symptoms.push(format!("High concurrency: {} active spans", vitals.active_spans));
            if status == HealthStatus::Healthy {
                status = HealthStatus::Degraded;
            }
        }

        if vitals.average_latency_ms > 500 {
            symptoms.push(format!("High latency: {}ms average", vitals.average_latency_ms));
            if status == HealthStatus::Healthy {
                status = HealthStatus::Degraded;
            }
        }

        // --- Gallifrey Entropy Checks ---
        let entropy = self.gallifrey.entropy().ok();
        let heatmap = self.gallifrey.temporal_heatmap().ok().map(|h| h.render_ascii());

        if let Some(ref e) = entropy {
            if e.stability_score < 0.5 {
                symptoms.push(format!(
                    "System instability detected (Score: {:.2}). High temporal drift.",
                    e.stability_score
                ));
                if status != HealthStatus::Critical {
                    status = HealthStatus::Degraded;
                }
            }
            if e.retcon_count > 10 {
                symptoms.push(format!("High retcon rate: {} history rewrites", e.retcon_count));
            }
            if e.prophecy_count > 10 {
                symptoms.push(format!("High prophecy rate: {} future predictions", e.prophecy_count));
            }
        }

        // --- Causality & Prescription ---

        let mut root_causes = Vec::new();
        let mut prescription = None;

        if status != HealthStatus::Healthy {
            if vitals.error_count > 0 {
                root_causes.push("Recent system errors (check logs)".to_string());
            }
            if let Some(ref e) = entropy {
                if e.stability_score < 0.5 {
                    root_causes.push("Temporal drift (system clock or async issues)".to_string());
                }
            }

            prescription = Some(Prescription {
                description: "Review system logs and check temporal consistency.".to_string(),
                auto_fix_command: None,
            });
        }

        Diagnosis {
            status,
            symptoms,
            entropy,
            heatmap,
            root_causes,
            prescription,
        }
    }

    /// Generate a full human-readable report.
    #[allow(clippy::unused_async)]
    pub async fn report(&self) -> String {
        let diagnosis = self.diagnose().await;
        let mut output = String::new();

        writeln!(&mut output, "\n🩺 SYSTEM DOCTOR REPORT 🩺").ok();
        writeln!(&mut output, "==========================").ok();
        writeln!(&mut output, "Status: {:?}", diagnosis.status).ok();

        if !diagnosis.symptoms.is_empty() {
            writeln!(&mut output, "\nSymptoms:").ok();
            for symptom in &diagnosis.symptoms {
                writeln!(&mut output, " - {}", symptom).ok();
            }
        }

        if let Some(ref entropy) = diagnosis.entropy {
            writeln!(&mut output, "\n{}", entropy.report()).ok();
        }

        if let Some(ref heatmap) = diagnosis.heatmap {
            writeln!(&mut output, "\nTemporal Heatmap:").ok();
            writeln!(&mut output, "{}", heatmap).ok();
        }

        if !diagnosis.root_causes.is_empty() {
            writeln!(&mut output, "\nPossible Causes:").ok();
            for cause in &diagnosis.root_causes {
                writeln!(&mut output, " - {}", cause).ok();
            }
        }

        if let Some(rx) = diagnosis.prescription {
            writeln!(&mut output, "\nPrescription:").ok();
            writeln!(&mut output, " {}", rx.description).ok();
            if let Some(cmd) = rx.auto_fix_command {
                writeln!(&mut output, " Suggested Command: {}", cmd).ok();
            }
        }

        if diagnosis.status == HealthStatus::Healthy {
            writeln!(&mut output, "\nSystem is operating within normal parameters.").ok();
        }

        output
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tardis_telemetry::userspace::layer::{EventData, SpanData};
    use tardis_telemetry::types::{Subsystem, TraceId, SpanId};
    use std::time::Instant;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_doctor_diagnosis() {
        // Setup
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Some(Arc::clone(&telemetry)), gallifrey);

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
        assert!(diagnosis.symptoms.iter().any(|s| s.contains("High latency")));

        // Report generation
        let report = doctor.report().await;
        println!("{}", report);
        assert!(report.contains("SYSTEM DOCTOR REPORT"));
    }
}
