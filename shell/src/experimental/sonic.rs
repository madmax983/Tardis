//! The Sonic Screwdriver 🛠️
//!
//! "It's a scientific instrument, not a magic wand!"
//!
//! A CLI tool for file repair and analysis, leveraging `PsychicPaper`
//! for heuristic parsing and validation.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tardis_chronos::experimental::doctor::{HealthStatus, SystemDoctor};
use tardis_chronos::experimental::psychic_paper::{Intent, PsychicPaper};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;
use tardis_vortex::{ModelHandle, Vortex};

/// The Sonic Screwdriver.
#[derive(Debug, Default)]
pub struct SonicScrewdriver {
    paper: PsychicPaper,
    telemetry: Option<Arc<TelemetryStore>>,
    gallifrey: Option<Arc<Gallifrey>>,
    vortex: Option<Arc<Vortex>>,
    model_handle: Option<ModelHandle>,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub const fn new(
        telemetry: Option<Arc<TelemetryStore>>,
        gallifrey: Option<Arc<Gallifrey>>,
        vortex: Option<Arc<Vortex>>,
        model_handle: Option<ModelHandle>,
    ) -> Self {
        Self {
            paper: PsychicPaper::new(),
            telemetry,
            gallifrey,
            vortex,
            model_handle,
        }
    }

    /// Run a system diagnosis.
    ///
    /// # Errors
    ///
    /// Returns an error if telemetry or gallifrey is not available.
    pub async fn diagnose(&self) -> Result<String> {
        let (Some(telemetry), Some(gallifrey)) = (&self.telemetry, &self.gallifrey) else {
            return Ok("⚠️  Sonic Screwdriver needs telemetry and gallifrey to diagnose system health.\n   (Telemetry offline)".to_string());
        };

        let mut doctor = SystemDoctor::new(telemetry.clone(), gallifrey.clone());

        if let Some(vortex) = &self.vortex {
            doctor = doctor.with_vortex(vortex.clone());
        }

        if let Some(handle) = self.model_handle {
            doctor = doctor.with_model(handle);
        }

        let diagnosis = doctor.diagnose().await;

        let mut report = String::new();
        let _ = writeln!(report, "🔍  SYSTEM DIAGNOSIS");
        let _ = writeln!(report, "====================\n");

        match diagnosis.status {
            HealthStatus::Healthy => {
                let _ = writeln!(report, "Status: HEALTHY 🟢");
            }
            HealthStatus::Degraded => {
                let _ = writeln!(report, "Status: DEGRADED 🟡");
            }
            HealthStatus::Critical => {
                let _ = writeln!(report, "Status: CRITICAL 🔴");
            }
        }

        if !diagnosis.symptoms.is_empty() {
            let _ = writeln!(report, "\nSymptoms:");
            for symptom in &diagnosis.symptoms {
                let _ = writeln!(report, " - {symptom}");
            }
        }

        if !diagnosis.root_causes.is_empty() {
            let _ = writeln!(report, "\nPotential Causes:");
            for cause in &diagnosis.root_causes {
                let _ = writeln!(report, " - {cause}");
            }
        }

        if let Some(prescription) = diagnosis.prescription {
            let _ = writeln!(report, "\nPrescription:");
            let _ = writeln!(report, " 💊 {}", prescription.description);
            if let Some(cmd) = prescription.auto_fix_command {
                let _ = writeln!(report, "    Run: {cmd}");
            }
        }

        Ok(report)
    }

    /// "Buzz" the sonic screwdriver.
    ///
    /// Just for fun.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn buzz(&self) -> String {
        "🔊 *Whirrrrrr-buzz-click-whirrrrrr*".to_string()
    }

    /// Inspect a file and return a diagnosis.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn inspect(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let size = content.len();
        let lines = content.lines().count();

        let mut diagnosis = format!(
            "File: {}\nSize: {size} bytes\nLines: {lines}\n",
            path.display()
        );

        // Try to interpret as JSON
        match self.paper.interpret(&content, Intent::Json) {
            Ok(_) => diagnosis.push_str("Type: Valid JSON\nStatus: Healthy 🟢"),
            Err(_) => {
                // Try other formats
                if self.paper.interpret(&content, Intent::KeyValue).is_ok() {
                    diagnosis.push_str("Type: Key-Value Pairs\nStatus: Healthy 🟢");
                } else if self.paper.interpret(&content, Intent::List).is_ok() {
                    diagnosis.push_str("Type: List\nStatus: Healthy 🟢");
                } else {
                    match self.paper.interpret(&content, Intent::Auto) {
                        Ok(val) => {
                            if val.is_string() {
                                diagnosis.push_str("Type: Unknown / Text\nStatus: Ambiguous 🟡");
                            } else {
                                diagnosis.push_str(
                                    "Type: Structured (Auto-detected)\nStatus: Healthy 🟢",
                                );
                            }
                        }
                        Err(e) => {
                            let _ =
                                write!(diagnosis, "Type: Unknown\nStatus: Broken 🔴\nError: {e}");
                        }
                    }
                }
            }
        }

        Ok(diagnosis)
    }

    /// Attempt to repair a broken file.
    ///
    /// Currently supports:
    /// - Fixing malformed JSON (via lenient parsing + pretty print).
    /// - Normalizing KV/List formats.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or written.
    pub fn repair(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Create backup
        let backup_path = path.with_extension("bak");
        fs::write(&backup_path, &content)
            .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;

        // Attempt repair via interpretation
        // We use Auto intent to let PsychicPaper figure it out
        let interpreted = self
            .paper
            .interpret(&content, Intent::Auto)
            .map_err(|e| anyhow::anyhow!("Failed to interpret file: {e}"))?;

        // If it's a string, we probably didn't parse anything structured
        if let Value::String(_) = interpreted {
            return Ok(format!(
                "Could not identify structure to repair. Backup created at {}",
                backup_path.display()
            ));
        }

        // Write back as pretty-printed JSON
        let fixed_content = serde_json::to_string_pretty(&interpreted)?;
        fs::write(path, fixed_content)
            .with_context(|| format!("Failed to write fixed file: {}", path.display()))?;

        Ok(format!(
            "Repaired file: {}\nBackup saved to: {}\nFormat: JSON (Normalized)",
            path.display(),
            backup_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_file(content: &str) -> (std::path::PathBuf, impl FnOnce()) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sonic_test_{}.tmp", nanos));
        let mut file = File::create(&path).unwrap();
        write!(file, "{}", content).unwrap();

        let path_clone = path.clone();
        let cleanup = move || {
            let _ = fs::remove_file(path_clone.clone());
            let backup = path_clone.with_extension("bak");
            if backup.exists() {
                let _ = fs::remove_file(backup);
            }
        };
        (path, cleanup)
    }

    #[test]
    fn test_inspect_json() -> Result<()> {
        let (path, cleanup) = create_temp_file(r#"{"key": "value"}"#);

        let sonic = SonicScrewdriver::new(None, None, None, None);
        let diagnosis = sonic.inspect(&path)?;
        assert!(diagnosis.contains("Valid JSON"));

        cleanup();
        Ok(())
    }

    #[test]
    fn test_repair_malformed_json() -> Result<()> {
        // PsychicPaper can handle markdown code blocks or messy JSON
        let (path, cleanup) = create_temp_file("```json\n{\"key\": \"value\"}\n```");

        let sonic = SonicScrewdriver::new(None, None, None, None);
        let report = sonic.repair(&path)?;

        assert!(report.contains("Repaired file"));

        let content = fs::read_to_string(&path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert_eq!(json["key"], "value");

        // Check backup
        let backup_path = path.with_extension("bak");
        assert!(backup_path.exists());

        cleanup();
        Ok(())
    }

    #[tokio::test]
    async fn test_doctor_integration() {
        use std::collections::HashMap;
        use tardis_telemetry::types::{Level, Subsystem, TraceId};
        use tardis_telemetry::userspace::layer::EventData;

        // Setup dependencies
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let vortex = Arc::new(Vortex::new().unwrap());

        // Mock model loading
        let mock_handle = ModelHandle::new(42);
        vortex.set_mock_load_model(Box::new(move |_path, _config| Ok(mock_handle)));

        // Load a "dummy" model to get a valid handle in registry
        let handle = vortex
            .load_model("/dummy/model", tardis_vortex::ModelLoadConfig::default())
            .await
            .unwrap();

        // Mock inference
        let expected_diagnosis = "AI Diagnosis: Capacitor fluxing";
        vortex.set_mock_inference(Box::new(move |_handle, _prompt, _params| {
            Ok(expected_diagnosis.to_string())
        }));

        // Inject an error to trigger diagnosis
        let error_event = EventData {
            span_id: None,
            trace_id: TraceId::generate(),
            timestamp: chrono::Utc::now(),
            level: Level::Error,
            message: "Flux capacitor unstable".to_string(),
            fields: HashMap::new(),
            subsystem: Subsystem::Kernel,
        };
        telemetry.record_event(error_event).await.unwrap();

        // Run Sonic Screwdriver
        let sonic =
            SonicScrewdriver::new(Some(telemetry), Some(gallifrey), Some(vortex), Some(handle));

        let report = sonic.diagnose().await.unwrap();

        // Verify report contains AI diagnosis
        assert!(report.contains("AI Diagnosis: Capacitor fluxing"));
        assert!(report.contains("Status: DEGRADED"));
    }
}
