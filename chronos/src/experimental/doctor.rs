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
use tracing::warn;

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
    #[allow(dead_code)]
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

    /// Check temporal health by comparing recent snapshots.
    ///
    /// Looks for regression in process memory usage over the last hour.
    #[allow(clippy::cast_precision_loss)]
    fn check_temporal_health(&self) -> Option<Vec<String>> {
        let store = self.gallifrey.system_state();

        // Get all snapshots
        let snapshots = match store.list_snapshots() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to list snapshots: {}", e);
                return None;
            }
        };

        if snapshots.len() < 2 {
            return None;
        }

        // Get latest snapshot
        let latest = snapshots.last()?;

        // Find a comparison snapshot (at least 5 minutes ago, up to 1 hour)
        let now = latest.timestamp;
        let window = Duration::minutes(5);

        // Find the most recent snapshot that is at least 5 minutes older than 'latest'
        let comparison = snapshots.iter().rev().find(|s| {
            let age = now - s.timestamp;
            age >= window
        });

        let comparison = if let Some(s) = comparison {
            s
        } else {
            // If no snapshot is old enough, maybe just take the oldest one if it's distinct
            let first = snapshots.first()?;
            if first.id == latest.id {
                return None;
            }
            first
        };

        let mut symptoms = Vec::new();

        // Compare Process Memory
        for (pid, proc_latest) in &latest.state.processes {
            if let Some(proc_old) = comparison.state.processes.get(pid) {
                // Check for significant memory growth (> 20MB and > 20%)
                // We cast to i64 to avoid overflow if memory shrinks (though usually usize is safe, diff can be negative conceptually)
                // But memory_bytes is u64.
                let mem_latest = proc_latest.memory_bytes;
                let mem_old = proc_old.memory_bytes;

                if mem_latest > mem_old {
                    let diff = mem_latest - mem_old;
                    let threshold_bytes = 20 * 1024 * 1024; // 20 MB

                    if diff > threshold_bytes {
                        let percent_growth = (diff as f64 / mem_old as f64) * 100.0;
                        if percent_growth > 20.0 {
                            symptoms.push(format!(
                                 "Memory leak detected: process '{}' (PID {}) grew by {:.1}% ({:.1} MB) in {:.1} min",
                                 proc_latest.name,
                                 pid,
                                 percent_growth,
                                 diff as f64 / 1_024.0 / 1_024.0,
                                 (latest.timestamp - comparison.timestamp).num_minutes()
                             ));
                        }
                    }
                }
            }
        }

        Some(symptoms)
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

        // Check Temporal Health (Gallifrey)
        if let Some(mut temporal_symptoms) = self.check_temporal_health() {
            if !temporal_symptoms.is_empty() {
                // If we found temporal issues, upgrade status if currently healthy
                if status == HealthStatus::Healthy {
                    status = HealthStatus::Degraded;
                }
                symptoms.append(&mut temporal_symptoms);
            }
        }

        // Correlate with system changes (simple heuristic for now)
        // In a real version, we'd ask Gallifrey for recent "SystemState" changes
        // and ask Vortex to find causality.
        let root_causes = if status == HealthStatus::Healthy {
            Vec::new()
        } else {
            // Mock causality
            vec!["Possible recent configuration change or high load".to_string()]
        };

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
    use tardis_common::id::SnapshotId;
    use tardis_gallifrey::domain::{ProcessState, Snapshot, SnapshotTrigger, SystemState};
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
    async fn test_doctor_temporal_health() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), Arc::clone(&gallifrey));

        // 1. Create a "Past" snapshot (1 hour ago)
        let past_time = Utc::now() - Duration::hours(1);
        let mut processes_past = HashMap::new();
        processes_past.insert(
            101,
            ProcessState {
                pid: 101,
                name: "leaky_app".to_string(),
                status: "Running".to_string(),
                memory_bytes: 100 * 1024 * 1024, // 100 MB
                cpu_percent: 5.0,
            },
        );

        let snapshot_past = Snapshot {
            id: SnapshotId::new(),
            name: "past".to_string(),
            timestamp: past_time,
            trigger: SnapshotTrigger::Scheduled,
            state: SystemState {
                processes: processes_past,
                config: HashMap::new(),
                files: HashMap::new(),
            },
            checksum: "dummy".to_string(),
        };

        // 2. Create a "Present" snapshot (Now)
        // leaky_app has grown to 200 MB (+100%)
        let mut processes_now = HashMap::new();
        processes_now.insert(
            101,
            ProcessState {
                pid: 101,
                name: "leaky_app".to_string(),
                status: "Running".to_string(),
                memory_bytes: 200 * 1024 * 1024, // 200 MB
                cpu_percent: 10.0,
            },
        );

        let snapshot_now = Snapshot {
            id: SnapshotId::new(),
            name: "present".to_string(),
            timestamp: Utc::now(),
            trigger: SnapshotTrigger::Manual,
            state: SystemState {
                processes: processes_now,
                config: HashMap::new(),
                files: HashMap::new(),
            },
            checksum: "dummy".to_string(),
        };

        // Inject snapshots
        gallifrey
            .system_state()
            .insert_snapshot(snapshot_past)
            .unwrap();
        gallifrey
            .system_state()
            .insert_snapshot(snapshot_now)
            .unwrap();

        // 3. Diagnose
        let diagnosis = doctor.diagnose().await;

        // 4. Verify
        assert_ne!(diagnosis.status, HealthStatus::Healthy);
        assert!(diagnosis
            .symptoms
            .iter()
            .any(|s| s.contains("Memory leak detected") && s.contains("leaky_app")));
    }
}
