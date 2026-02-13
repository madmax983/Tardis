//! The Sonic Screwdriver 🛠️
//!
//! "It's a scientific instrument, not a magic wand!"
//!
//! A CLI tool for file repair and analysis, leveraging PsychicPaper
//! for heuristic parsing and validation.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use tardis_chronos::experimental::doctor::SystemDoctor;
use tardis_chronos::experimental::psychic_paper::{Intent, PsychicPaper};

/// The Sonic Screwdriver.
#[derive(Debug, Default)]
pub struct SonicScrewdriver {
    paper: PsychicPaper,
    doctor: Option<SystemDoctor>,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub const fn new(doctor: Option<SystemDoctor>) -> Self {
        Self {
            paper: PsychicPaper::new(),
            doctor,
        }
    }

    /// Run a system diagnosis.
    ///
    /// # Errors
    ///
    /// Returns an error if the doctor is not available or diagnosis fails.
    pub async fn diagnose_system(&self) -> Result<String> {
        let Some(doctor) = &self.doctor else {
            return Ok("Sonic Screwdriver Error: System Doctor interface not initialized. \
                       (Telemetry might be disabled or unavailable)"
                .to_string());
        };

        let diagnosis = doctor.diagnose().await;

        let status_icon = match diagnosis.status {
            tardis_chronos::experimental::doctor::HealthStatus::Healthy => "🟢",
            tardis_chronos::experimental::doctor::HealthStatus::Degraded => "🟡",
            tardis_chronos::experimental::doctor::HealthStatus::Critical => "🔴",
        };

        let mut report = String::new();
        writeln!(report, "\n🔍 Sonic Diagnostics Report 🔍")?;
        writeln!(report, "=============================\n")?;

        writeln!(report, "Status: {status_icon} {:?}\n", diagnosis.status)?;

        if let Some(summary) = &diagnosis.summary {
            writeln!(report, "Summary:\n  {summary}\n")?;
        }

        if !diagnosis.symptoms.is_empty() {
            writeln!(report, "Symptoms:")?;
            for symptom in &diagnosis.symptoms {
                writeln!(report, "  - {symptom}")?;
            }
            writeln!(report)?;
        }

        if !diagnosis.root_causes.is_empty() {
            writeln!(report, "Potential Root Causes:")?;
            for cause in &diagnosis.root_causes {
                writeln!(report, "  - {cause}")?;
            }
            writeln!(report)?;
        }

        if let Some(prescription) = &diagnosis.prescription {
            writeln!(report, "Prescription:\n  {}", prescription.description)?;
            if let Some(cmd) = &prescription.auto_fix_command {
                writeln!(report, "  Suggested Command: {cmd}")?;
            }
        } else {
            writeln!(report, "Prescription:\n  No action required. Carry on!")?;
        }

        Ok(report)
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

        let sonic = SonicScrewdriver::new(None);
        let diagnosis = sonic.inspect(&path)?;
        assert!(diagnosis.contains("Valid JSON"));

        cleanup();
        Ok(())
    }

    #[test]
    fn test_repair_malformed_json() -> Result<()> {
        // PsychicPaper can handle markdown code blocks or messy JSON
        let (path, cleanup) = create_temp_file("```json\n{\"key\": \"value\"}\n```");

        let sonic = SonicScrewdriver::new(None);
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
    async fn test_sonic_diagnose() {
        use std::sync::Arc;
        use tardis_gallifrey::Gallifrey;
        use tardis_telemetry::gallifrey::TelemetryStore;

        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        let doctor = SystemDoctor::new(Arc::clone(&telemetry), gallifrey);

        let sonic = SonicScrewdriver::new(Some(doctor));
        let report = sonic.diagnose_system().await.unwrap();

        assert!(report.contains("Sonic Diagnostics Report"));
        assert!(report.contains("Status: 🟢 Healthy"));
    }
}
