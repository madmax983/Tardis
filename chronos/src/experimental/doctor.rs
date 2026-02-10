//! System Doctor: Diagnoses system health by correlating telemetry and temporal anomalies.
//!
//! "I am the Doctor, and I save people." - The Doctor
//!
//! This module provides a `SystemDoctor` that analyzes:
//! 1. Recent errors from `TelemetryStore`.
//! 2. Temporal instability (Retcons/Prophecies) from `Gallifrey`.
//! 3. Correlates them to find if "rewriting history" caused errors.

#![cfg(feature = "nova")]

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tardis_gallifrey::experimental::entropy::EntropyGauge;
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_telemetry::Level;

/// A diagnosis of the system's health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Health score (0.0 to 1.0).
    pub health_score: f64,
    /// List of symptoms found.
    pub symptoms: Vec<String>,
    /// Probable cause of the issues.
    pub probable_cause: Option<String>,
    /// Prescriptive advice.
    pub prescription: String,
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

    /// Perform a full system diagnosis.
    ///
    /// # Errors
    ///
    /// Returns an error if the diagnosis fails.
    #[allow(clippy::cast_precision_loss)]
    pub async fn diagnose(&self) -> Result<Diagnosis, String> {
        let now = Utc::now();
        let lookback = Duration::minutes(5);
        let from = now - lookback;

        // 1. Check Telemetry for Errors
        // We look for spans with Level::Error or events that look like errors
        // TelemetryStore doesn't index by Level directly, so we scan recent spans.
        let spans = self.telemetry.spans_in_range(from, now);
        let error_count = spans
            .iter()
            .filter(|s| s.data.level == Level::Error)
            .count();

        // 2. Check Gallifrey for Entropy (Retcons)
        let knowledge = self.gallifrey.knowledge();
        let entropy = EntropyGauge::measure_global(&knowledge)
            .map_err(|e| format!("Failed to measure entropy: {e}"))?;

        // 3. Analyze Symptoms
        let mut symptoms = Vec::new();
        if error_count > 0 {
            symptoms.push(format!("Found {error_count} errors in the last 5 minutes."));
        }
        if entropy.retcon_count > 0 {
            symptoms.push(format!(
                "Detected {} Retcons (history rewrites).",
                entropy.retcon_count
            ));
        }
        if entropy.stability_score < 0.8 {
            symptoms.push(format!(
                "System stability is low ({:.2}).",
                entropy.stability_score
            ));
        }

        // 4. Formulate Diagnosis
        let health_score = if error_count == 0 && entropy.stability_score > 0.9 {
            1.0
        } else {
            let error_penalty = (error_count as f64 * 0.1).min(0.5);
            (entropy.stability_score - error_penalty).max(0.0)
        };

        let (probable_cause, prescription) = if error_count > 0 && entropy.retcon_count > 0 {
            (
                Some("Temporal Dissonance: Recent history rewrites correlate with system errors.".to_string()),
                "Stop changing the past! Stabilize the timeline before attempting further operations.".to_string(),
            )
        } else if error_count > 0 {
            (
                Some("Runtime Exception: Standard system failure.".to_string()),
                "Check the logs for specific error traces.".to_string(),
            )
        } else if entropy.stability_score < 0.5 {
            (
                Some("Temporal Instability: High rate of retcons/prophecies.".to_string()),
                "The timeline is fracturing. Reduce the frequency of bi-temporal updates.".to_string(),
            )
        } else {
            (
                None,
                "Take two jelly babies and call me in the morning.".to_string(),
            )
        };

        Ok(Diagnosis {
            health_score,
            symptoms,
            probable_cause,
            prescription,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Instant;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::{BiTemporalInterval, TimeRange};
    use tardis_telemetry::types::{SpanId, Subsystem, TraceId};
    use tardis_telemetry::userspace::layer::SpanData;

    #[tokio::test]
    async fn test_doctor_diagnosis_healthy() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(telemetry, gallifrey);

        let diagnosis = doctor.diagnose().await.unwrap();
        assert!(diagnosis.health_score > 0.9);
        assert_eq!(diagnosis.symptoms.len(), 0);
        assert_eq!(
            diagnosis.prescription,
            "Take two jelly babies and call me in the morning."
        );
    }

    #[tokio::test]
    async fn test_doctor_diagnosis_sick() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());

        // Inject an error span
        let span = SpanData {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_id: None,
            name: "Critical Failure".to_string(),
            subsystem: Subsystem::Vortex,
            start_instant: Instant::now(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            attributes: HashMap::new(),
            level: Level::Error,
        };
        telemetry.record_span(span).await.unwrap();

        // Inject a Retcon (Valid < Transaction)
        // We need the valid time to be strictly before transaction time
        // And transaction time to be "now" so it is counted in entropy measure
        let now = Utc::now();
        let past = now - Duration::hours(1);

        let entity = tardis_common::domain::Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: "Retconned Entity".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval {
                valid_time: TimeRange::starting_at(past),
                transaction_time: TimeRange::starting_at(now),
            },
            source: None,
        };
        gallifrey.insert(entity).await.unwrap();

        let doctor = SystemDoctor::new(telemetry, gallifrey);
        let diagnosis = doctor.diagnose().await.unwrap();

        // Retcon + Error = Temporal Dissonance
        assert!(diagnosis.health_score < 1.0);
        assert!(diagnosis.symptoms.len() >= 2); // Error + Retcon
        assert_eq!(
            diagnosis.probable_cause,
            Some("Temporal Dissonance: Recent history rewrites correlate with system errors.".to_string())
        );
    }
}
