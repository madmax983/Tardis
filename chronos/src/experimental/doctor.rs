//! The System Doctor 🩺
//!
//! A self-diagnostic tool that correlates telemetry data (symptoms)
//! with system state changes (causes) to prescribe fixes.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tardis_gallifrey::Gallifrey;
use tardis_gallifrey::experimental::entropy::EntropyGauge;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::types::Level;
use chrono::{Duration, Utc};

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
    pub fn new(telemetry: Arc<TelemetryStore>, gallifrey: Arc<Gallifrey>) -> Self {
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

        // Check Temporal Entropy (Nova Feature)
        if let Ok(entropy) = EntropyGauge::measure_global(&self.gallifrey.knowledge()) {
            if entropy.stability_score < 0.5 {
                status = match status {
                    HealthStatus::Healthy | HealthStatus::Degraded => HealthStatus::Degraded,
                    HealthStatus::Critical => HealthStatus::Critical,
                };

                // If it's really bad
                if entropy.stability_score < 0.2 {
                    status = HealthStatus::Critical;
                }

                symptoms.push(format!(
                    "High temporal instability: {:.2} ({} retcons)",
                    entropy.stability_score,
                    entropy.retcon_count
                ));
            }
        }

        // Correlate with system changes (simple heuristic for now)
        // In a real version, we'd ask Gallifrey for recent "SystemState" changes
        // and ask Vortex to find causality.
        let mut root_causes = Vec::new();
        let mut prescription = None;

        if status != HealthStatus::Healthy {
            // Mock causality for non-entropy issues
            if symptoms.iter().any(|s| s.contains("errors") || s.contains("latency")) {
                root_causes.push("Possible recent configuration change or high load".to_string());
                prescription = Some(Prescription {
                    description: "Check recent system state changes and error logs.".to_string(),
                    auto_fix_command: None,
                });
            }

            // Prescription for entropy
            if symptoms.iter().any(|s| s.contains("temporal instability")) {
                root_causes.push("Excessive retroactive updates (retcons) detected".to_string());
                let entropy_fix = Prescription {
                    description: "Reduce retroactive updates to minimize entropy.".to_string(),
                    auto_fix_command: None,
                };
                // Prioritize entropy fix if critical
                if prescription.is_none() || status == HealthStatus::Critical {
                    prescription = Some(entropy_fix);
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
    use tardis_telemetry::userspace::layer::{EventData, SpanData};
    use tardis_telemetry::types::{Subsystem, TraceId, SpanId};
    use std::time::Instant;
    use std::collections::HashMap;
    use tardis_gallifrey::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};

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
        assert!(diagnosis.symptoms.iter().any(|s| s.contains("High latency")));
    }

    #[tokio::test]
    async fn test_doctor_entropy_diagnosis() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), Arc::clone(&gallifrey));

        // Inject massive retcons to create entropy
        let store = gallifrey.knowledge();
        let now = Utc::now();
        let hour = Duration::hours(1);

        for i in 0..10 {
            // Retcon: Valid Time (1 hour ago) < Transaction Time (Now)
            let entity = Entity {
                id: EntityId::new(),
                entity_type: "Retcon".to_string(),
                name: format!("Retcon-{}", i),
                properties: HashMap::new(),
                embedding: None,
                temporal: BiTemporalInterval {
                    valid_time: TimeRange::starting_at(now - hour),
                    transaction_time: TimeRange::starting_at(now),
                },
                source: None,
            };
            store.insert_entity(entity).unwrap();
        }

        // Diagnose
        let diagnosis = doctor.diagnose().await;

        // Should be at least degraded
        assert!(diagnosis.status != HealthStatus::Healthy);

        // Should complain about instability
        let has_instability = diagnosis.symptoms.iter().any(|s| s.contains("High temporal instability"));
        assert!(has_instability, "Doctor should detect temporal instability. Symptoms: {:?}", diagnosis.symptoms);

        // Should prescribe reducing retcons
        assert!(diagnosis.prescription.is_some());
        assert!(diagnosis.prescription.unwrap().description.contains("Reduce retroactive updates"));
    }
}
