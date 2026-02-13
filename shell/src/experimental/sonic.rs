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

/// The Sonic Screwdriver.
#[derive(Debug, Default)]
pub struct SonicScrewdriver {
    paper: PsychicPaper,
    telemetry: Option<Arc<TelemetryStore>>,
    gallifrey: Option<Arc<Gallifrey>>,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub const fn new(
        telemetry: Option<Arc<TelemetryStore>>,
        gallifrey: Option<Arc<Gallifrey>>,
    ) -> Self {
        Self {
            paper: PsychicPaper::new(),
            telemetry,
            gallifrey,
        }
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

        let mut diagnosis = format!("File: {}\nSize: {size} bytes\nLines: {lines}\n", path.display());

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
                            let _ = write!(diagnosis, "Type: Unknown\nStatus: Broken 🔴\nError: {e}");
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
        fs::write(&backup_path, &content).with_context(|| {
            format!("Failed to create backup: {}", backup_path.display())
        })?;

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

    /// Run a system-wide diagnosis using the System Doctor.
    ///
    /// # Errors
    ///
    /// Returns an error if the diagnosis fails.
    #[allow(clippy::unused_async)]
    pub async fn diagnose_system(&self) -> Result<String> {
        if let (Some(telemetry), Some(gallifrey)) = (&self.telemetry, &self.gallifrey) {
            let doctor = SystemDoctor::new(Arc::clone(telemetry), Arc::clone(gallifrey));
            let diagnosis = doctor.diagnose().await;

            let mut report = String::new();
            report.push_str("🏥 System Diagnosis Report\n");
            report.push_str("=========================\n\n");

            match diagnosis.status {
                HealthStatus::Healthy => report.push_str("Status: Healthy 🟢\n"),
                HealthStatus::Degraded => report.push_str("Status: Degraded 🟡\n"),
                HealthStatus::Critical => report.push_str("Status: Critical 🔴\n"),
            }

            if !diagnosis.symptoms.is_empty() {
                report.push_str("\nSymptoms:\n");
                for symptom in &diagnosis.symptoms {
                    let _ = writeln!(report, " - {symptom}");
                }
            }

            if !diagnosis.root_causes.is_empty() {
                report.push_str("\nPotential Root Causes:\n");
                for cause in &diagnosis.root_causes {
                    let _ = writeln!(report, " - {cause}");
                }
            }

            if let Some(rx) = diagnosis.prescription {
                report.push_str("\nPrescription:\n");
                let _ = writeln!(report, " 💊 {}", rx.description);
                if let Some(cmd) = rx.auto_fix_command {
                    let _ = writeln!(report, "    Run: {cmd}");
                }
            } else {
                report.push_str("\nNo action required. Keep flying! 🚀\n");
            }

            Ok(report)
        } else {
            Ok("⚠️ Sonic Screwdriver is disconnected from system mainframe (Telemetry/Gallifrey missing).".to_string())
        }
    }

    /// Make the sonic screwdriver buzz.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn buzz(&self) -> String {
        "🔊 Bzzzzzzzzzzzzzzzt! (It's working!)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tardis_telemetry::types::{Level, Subsystem, TraceId};
    use tardis_telemetry::userspace::layer::EventData;
    use std::collections::HashMap;
    use chrono::Utc;

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

        let sonic = SonicScrewdriver::new(None, None);
        let diagnosis = sonic.inspect(&path)?;
        assert!(diagnosis.contains("Valid JSON"));

        cleanup();
        Ok(())
    }

    #[test]
    fn test_repair_malformed_json() -> Result<()> {
        // PsychicPaper can handle markdown code blocks or messy JSON
        let (path, cleanup) = create_temp_file("```json\n{\"key\": \"value\"}\n```");

        let sonic = SonicScrewdriver::new(None, None);
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
    async fn test_diagnose_system() {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let sonic = SonicScrewdriver::new(Some(telemetry.clone()), Some(gallifrey.clone()));

        // Healthy
        let report = sonic.diagnose_system().await.unwrap();
        assert!(report.contains("Status: Healthy"));

        // Inject Error
        let error_event = EventData {
            span_id: None,
            trace_id: TraceId::generate(),
            timestamp: Utc::now(),
            level: Level::Error,
            message: "Something exploded".to_string(),
            fields: HashMap::new(),
            subsystem: Subsystem::Kernel,
        };
        // Inject 11 errors to trigger Critical
        for _ in 0..11 {
             telemetry.record_event(error_event.clone()).await.unwrap();
        }

        let report = sonic.diagnose_system().await.unwrap();
        assert!(report.contains("Status: Critical"));
        assert!(report.contains("Symptoms"));
    }
}
