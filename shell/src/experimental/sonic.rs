//! The Sonic Screwdriver 🔧
//!
//! A universal tool for scanning files, fixing data formats, and diagnosing system health.
//!
//! "It doesn't do wood."

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tardis_chronos::experimental::doctor::SystemDoctor;
use tardis_chronos::experimental::psychic_paper::{Intent, PsychicPaper};
use tardis_gallifrey::Gallifrey;
use tardis_telemetry::gallifrey::TelemetryStore;

/// The Sonic Screwdriver tool.
#[derive(Debug)]
pub struct SonicScrewdriver {
    gallifrey: Arc<Gallifrey>,
    telemetry: Option<Arc<TelemetryStore>>,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>, telemetry: Option<Arc<TelemetryStore>>) -> Self {
        Self {
            gallifrey,
            telemetry,
        }
    }

    /// Scan and "fix" a file by interpreting its content.
    ///
    /// Reads the file at `path`, attempts to parse it using `PsychicPaper`,
    /// and prints the structured representation (JSON).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn fix(&self, path: &str) -> Result<()> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(anyhow!("File not found: {}", path.display()));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read file: {}", e))?;

        println!("Scanning {}...", path.display());

        let paper = PsychicPaper::new();
        let result = paper.interpret(&content, Intent::Auto)
            .map_err(|e| anyhow!("Failed to interpret content: {}", e))?;

        println!("Structure detected!");
        println!("{}", serde_json::to_string_pretty(&result)?);

        Ok(())
    }

    /// Diagnose system health.
    ///
    /// uses `SystemDoctor` to check vitals and generate a diagnosis.
    ///
    /// # Errors
    ///
    /// Returns an error if telemetry is not available.
    pub async fn diagnose(&self) -> Result<()> {
        let Some(telemetry) = &self.telemetry else {
            return Err(anyhow!("Telemetry subsystem is offline. Cannot diagnose."));
        };

        println!("Running system diagnosis...");

        let doctor = SystemDoctor::new(telemetry.clone(), self.gallifrey.clone());
        let diagnosis = doctor.diagnose().await;

        println!("Health Status: {:?}", diagnosis.status);

        if !diagnosis.symptoms.is_empty() {
            println!("Symptoms:");
            for symptom in &diagnosis.symptoms {
                println!("  - {}", symptom);
            }
        }

        if !diagnosis.root_causes.is_empty() {
            println!("Potential Causes:");
            for cause in &diagnosis.root_causes {
                println!("  - {}", cause);
            }
        }

        if let Some(prescription) = &diagnosis.prescription {
            println!("Prescription:");
            println!("  {}", prescription.description);
            if let Some(cmd) = &prescription.auto_fix_command {
                println!("  Suggested command: {}", cmd);
            }
        } else {
            println!("No prescription needed.");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_fix_json() {
        let gallifrey = Arc::new(Gallifrey::new());
        let sonic = SonicScrewdriver::new(gallifrey, None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{ \"key\": \"value\" }}").unwrap();

        let result = sonic.fix(file.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_fix_broken_json_in_markdown() {
        let gallifrey = Arc::new(Gallifrey::new());
        let sonic = SonicScrewdriver::new(gallifrey, None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "```json\n{{ \"key\": \"value\" }}\n```").unwrap();

        let result = sonic.fix(file.path().to_str().unwrap());
        assert!(result.is_ok());
    }
}
