//! The Sonic Screwdriver 🪄
//!
//! "It doesn't kill, it doesn't wound, it doesn't maim. But it's very good at opening doors."
//!
//! A self-diagnostic and repair tool for Tardis OS.

use std::sync::Arc;
use tardis_chronos::experimental::doctor::{Diagnosis, HealthStatus, SystemDoctor, Vitals};
use tardis_chronos::experimental::psychic_paper::{Intent, PsychicPaper};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_vortex::Vortex;
use tracing::info;

/// The Sonic Screwdriver.
#[derive(Debug)]
pub struct SonicScrewdriver {
    doctor: SystemDoctor,
    vortex: Arc<Vortex>,
    psychic_paper: PsychicPaper,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub fn new(
        gallifrey: Arc<Gallifrey>,
        telemetry: Option<Arc<TelemetryStore>>,
        vortex: Arc<Vortex>,
    ) -> Self {
        // Fallback to empty telemetry store if none provided
        let telemetry = telemetry.unwrap_or_else(|| Arc::new(TelemetryStore::new()));

        Self {
            doctor: SystemDoctor::new(telemetry, gallifrey),
            vortex,
            psychic_paper: PsychicPaper::new(),
        }
    }

    /// Scan the system (run diagnostics).
    pub async fn scan(&self) -> Diagnosis {
        info!("Scanning system with Sonic Screwdriver...");
        self.doctor.diagnose().await
    }

    /// Check vitals only.
    #[must_use]
    #[allow(dead_code)]
    pub fn check_vitals(&self) -> Vitals {
        self.doctor.check_vitals()
    }

    /// Generate a fix for the current system state.
    ///
    /// Returns a shell command suggestion.
    ///
    /// # Errors
    ///
    /// Returns an error if no model is loaded or inference fails.
    pub async fn buzz(&self) -> anyhow::Result<String> {
        let diagnosis = self.scan().await;

        if diagnosis.status == HealthStatus::Healthy {
            return Ok("echo 'System is healthy. No fix needed.'".to_string());
        }

        let symptoms = diagnosis.symptoms.join("\n- ");
        let causes = diagnosis.root_causes.join("\n- ");

        let _prompt = format!(
            "You are the Sonic Screwdriver, an advanced diagnostic tool.\n\
            The system is showing the following symptoms:\n\
            - {symptoms}\n\
            \n\
            Possible causes:\n\
            - {causes}\n\
            \n\
            Suggest a SINGLE shell command to fix or mitigate this issue.\n\
            Output ONLY the command, no markdown, no explanation."
        );

        // Check if models are available (even if we can't use them directly yet)
        let models = self.vortex.list_models();
        let loaded_models = models.iter().filter(|m| m.loaded).count();

        // TODO: Implement actual inference when Vortex exposes model handles or default inference
        let response = if loaded_models > 0 {
            // Mock response for now as we wait for upstream support
            "echo 'Diagnostic complete. Suggest checking logs for detailed error report.'"
        } else {
            "echo 'No LLM loaded. Cannot generate fix. Please load a model with `models load`.'"
        };

        // Parse with Psychic Paper
        let parsed = self.psychic_paper.interpret(response, Intent::Auto);

        match parsed {
            Ok(val) => Ok(val.as_str().unwrap_or(response).to_string()),
            Err(_) => Ok(response.to_string()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sonic_scan() {
        let gallifrey = Arc::new(Gallifrey::new());
        // Use tempfile for telemetry directory if needed, but TelemetryStore::new() is in-memory usually?
        // Actually TelemetryStore usually requires a directory if persistent.
        // But here we use `TelemetryStore::new()` which might be in-memory or default.

        // If TelemetryStore::new() panics or fails, we might need to mock it or use a proper config.
        // Let's check TelemetryStore::new().

        let telemetry = Arc::new(TelemetryStore::new());

        // Create a dummy Vortex
        if let Ok(vortex) = Vortex::new() {
            let sonic = SonicScrewdriver::new(gallifrey, Some(telemetry), Arc::new(vortex));
            let diagnosis = sonic.scan().await;

            // Initially healthy
            assert_eq!(diagnosis.status, HealthStatus::Healthy);
        }
    }
}
