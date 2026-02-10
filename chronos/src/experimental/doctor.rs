//! The System Doctor: Diagnosing temporal drift and system health.
//!
//! "I am the Doctor. I am a Time Lord. I'm from the planet Gallifrey in the Constellation of Kasterborous."
//!
//! This module provides a diagnostic tool that analyzes the bi-temporal state of the system
//! and recent telemetry to determine the overall health of the Tardis OS.

use crate::error::ChronosResult;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_gallifrey::experimental::entropy::{EntropyGauge, SystemEntropy};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::{gallifrey::TelemetryStore, Level};
use tracing::info;

/// The health status of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is operating within normal parameters.
    Healthy,
    /// Minor temporal drift or warnings detected.
    Stable,
    /// Significant drift or errors detected.
    Unstable,
    /// Critical failure or temporal paradox imminent.
    Critical,
}

/// A report on the system's health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// The calculated health status.
    pub status: HealthStatus,
    /// The system entropy metrics.
    pub entropy: SystemEntropy,
    /// Number of errors detected in the recent window.
    pub error_count: usize,
    /// A human-readable diagnosis.
    pub diagnosis: String,
}

/// The System Doctor.
#[derive(Debug)]
pub struct SystemDoctor {
    gallifrey: Arc<Gallifrey>,
    telemetry: Option<Arc<TelemetryStore>>,
}

impl SystemDoctor {
    /// Create a new System Doctor.
    #[must_use]
    pub const fn new(
        gallifrey: Arc<Gallifrey>,
        telemetry: Option<Arc<TelemetryStore>>,
    ) -> Self {
        Self {
            gallifrey,
            telemetry,
        }
    }

    /// Diagnose the system's health.
    ///
    /// This analyzes:
    /// 1. Temporal entropy (drift between Valid and Transaction time).
    /// 2. Recent telemetry errors (if telemetry is available).
    ///
    /// # Errors
    ///
    /// Returns an error if the diagnosis fails (e.g., storage error).
    pub fn diagnose(&self) -> ChronosResult<HealthReport> {
        info!("The Doctor is diagnosing the system...");

        // 1. Measure Entropy
        let knowledge = self.gallifrey.knowledge();
        let entropy = EntropyGauge::measure_global(&knowledge)
            .map_err(|e| crate::error::ChronosError::Common(tardis_common::Error::Internal(e.to_string())))?;

        // 2. Check Telemetry (if available)
        let mut error_count = 0;
        if let Some(store) = &self.telemetry {
            let now = Utc::now();
            let hour_ago = now - Duration::hours(1);
            let events = store.events_in_range(hour_ago, now);

            error_count = events
                .iter()
                .filter(|e| e.data.level == Level::Error || e.data.level == Level::Warn)
                .count();
        }

        // 3. Calculate Status
        let status = Self::calculate_status(&entropy, error_count);

        // 4. Generate Diagnosis
        let diagnosis = Self::generate_diagnosis(status, &entropy, error_count);

        Ok(HealthReport {
            status,
            entropy,
            error_count,
            diagnosis,
        })
    }

    fn calculate_status(entropy: &SystemEntropy, error_count: usize) -> HealthStatus {
        if entropy.stability_score < 0.5 || error_count > 100 {
            HealthStatus::Critical
        } else if entropy.stability_score < 0.8 || error_count > 10 {
            HealthStatus::Unstable
        } else if entropy.stability_score < 0.95 || error_count > 0 {
            HealthStatus::Stable
        } else {
            HealthStatus::Healthy
        }
    }

    fn generate_diagnosis(
        status: HealthStatus,
        entropy: &SystemEntropy,
        error_count: usize,
    ) -> String {
        match status {
            HealthStatus::Healthy => "Systems nominal. Temporal flow is stable.".to_string(),
            HealthStatus::Stable => format!(
                "Minor fluctuations detected. {} errors in the last hour. Entropy score: {:.2}.",
                error_count, entropy.stability_score
            ),
            HealthStatus::Unstable => format!(
                "System unstable! {} errors detected. Temporal drift is significant ({:.0}ms avg).",
                error_count, entropy.average_drift_ms
            ),
            HealthStatus::Critical => format!(
                "CRITICAL WARNING: Paradox imminent! {} errors. Entropy stability at {:.1}%. Evacuate timeline immediately.",
                error_count, entropy.stability_score * 100.0
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tardis_common::domain::Entity;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_doctor_diagnosis() {
        // Setup Gallifrey
        let gallifrey = Arc::new(Gallifrey::new());

        // Insert some data to generate entropy
        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Test Entity".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };
        gallifrey.insert(entity).await.unwrap();

        // Setup Doctor (no telemetry for this test)
        let doctor = SystemDoctor::new(gallifrey, None);

        // Diagnose
        let report = doctor.diagnose().unwrap();

        assert_eq!(report.status, HealthStatus::Healthy);
        assert!(report.diagnosis.contains("Systems nominal"));
    }
}
