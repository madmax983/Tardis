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
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub const fn new(telemetry: Arc<TelemetryStore>, gallifrey: Arc<Gallifrey>) -> Self {
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
            .fold((0u64, 0u64), |(sum, count), dur| (sum + dur, count + 1));

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
        let mut prescription_desc = String::new();

        if status != HealthStatus::Healthy {
            let now = Utc::now();
            let lookback = Duration::minutes(15); // Look back 15 mins for changes

            // Query Gallifrey for changes
            match self.gallifrey.system_state().get_changes(now - lookback, now) {
                Ok(changes) => {
                    if changes.is_empty() {
                         root_causes.push("No recent configuration changes found.".to_string());
                    } else {
                        prescription_desc.push_str("Check recent system state changes: ");
                        for change in changes {
                            let cause = format!(
                                "Configuration change ({:?}) at {}: {}",
                                change.change_type,
                                change.timestamp.format("%H:%M:%S"),
                                change.path
                            );
                            root_causes.push(cause);
                        }
                    }
                }
                Err(e) => {
                    root_causes.push(format!("Failed to retrieve system changes: {e}"));
                }
            }

            // Check for memory regression (if we had current memory stats)
            // For now, we just list it as a check
            if prescription_desc.is_empty() {
                prescription_desc = "Check error logs and system load.".to_string();
            }
        }

        let prescription = (status != HealthStatus::Healthy).then_some(Prescription {
                description: prescription_desc,
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
            start_instant: Instant::now()
                .checked_sub(std::time::Duration::from_millis(1000))
                .unwrap(), // Started 1s ago
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
}
