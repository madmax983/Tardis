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
use tardis_vortex::{InferenceParams, Vortex};

/// The Sonic Screwdriver.
#[derive(Debug, Default)]
pub struct SonicScrewdriver {
    paper: PsychicPaper,
    telemetry: Option<Arc<TelemetryStore>>,
    gallifrey: Option<Arc<Gallifrey>>,
    vortex: Option<Arc<Vortex>>,
}

impl SonicScrewdriver {
    /// Create a new Sonic Screwdriver.
    #[must_use]
    pub const fn new(
        telemetry: Option<Arc<TelemetryStore>>,
        gallifrey: Option<Arc<Gallifrey>>,
        vortex: Option<Arc<Vortex>>,
    ) -> Self {
        Self {
            paper: PsychicPaper::new(),
            telemetry,
            gallifrey,
            vortex,
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

        let doctor = SystemDoctor::new(telemetry.clone(), gallifrey.clone());
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

        // Add AI insight
        if let Some(insight) = self.diagnose_with_ai(&report).await {
            let _ = writeln!(report, "\n🧠 Sonic's Insight:");
            let _ = writeln!(report, "   {}", insight.trim());
        }

        Ok(report)
    }

    async fn diagnose_with_ai(&self, context: &str) -> Option<String> {
        let vortex = self.vortex.as_ref()?;

        // Find a loaded model
        let loaded = vortex.list_loaded_models();
        let (handle, _) = loaded.first()?;

        let prompt = format!(
            "You are the Sonic Screwdriver, a tool of the Time Lords. \
            Analyze this system report and explain the situation briefly in character. \
            Be witty but helpful.\n\n\
            System Report:\n{context}\n\n\
            Diagnosis:"
        );

        let params = InferenceParams {
            max_tokens: 200,
            temperature: 0.7,
            ..InferenceParams::default()
        };

        vortex.infer(*handle, &prompt, params).await.ok()
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
    /// - **Nova:** AI-assisted repair if heuristic parsing fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or written.
    pub async fn repair(&self, path: &Path) -> Result<String> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Create backup
        let backup_path = path.with_extension("bak");
        tokio::fs::write(&backup_path, &content)
            .await
            .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;

        // Attempt repair via interpretation
        // We use Auto intent to let PsychicPaper figure it out
        let interpreted = self
            .paper
            .interpret(&content, Intent::Auto)
            .map_err(|e| anyhow::anyhow!("Failed to interpret file: {e}"))?;

        // If it's a string, we probably didn't parse anything structured
        if let Value::String(_) = interpreted {
            // Try AI repair if available
            if let Some(vortex) = &self.vortex {
                 if let Some((handle, _)) = vortex.list_loaded_models().first() {
                     let prompt = format!(
                         "Fix this broken file content. Return ONLY the fixed content, no markdown, no explanations.\n\nContent:\n{content}"
                     );

                     let params = InferenceParams {
                         max_tokens: 2048,
                         temperature: 0.1, // Deterministic
                         ..InferenceParams::default()
                     };

                     if let Ok(fixed) = vortex.infer(*handle, &prompt, params).await {
                         // Verify the fix is structured
                         if let Ok(verified) = self.paper.interpret(&fixed, Intent::Auto) {
                             if !verified.is_string() {
                                 let fixed_content = serde_json::to_string_pretty(&verified)?;
                                 tokio::fs::write(path, fixed_content)
                                     .await
                                     .with_context(|| format!("Failed to write fixed file: {}", path.display()))?;

                                 return Ok(format!(
                                     "Repaired file using Vortex AI 🌪️\nBackup saved to: {}\nFormat: Auto-detected",
                                     backup_path.display()
                                 ));
                             }
                         }
                     }
                 }
            }

            return Ok(format!(
                "Could not identify structure to repair. Backup created at {}",
                backup_path.display()
            ));
        }

        // Write back as pretty-printed JSON
        let fixed_content = serde_json::to_string_pretty(&interpreted)?;
        tokio::fs::write(path, fixed_content)
            .await
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

        let sonic = SonicScrewdriver::new(None, None, None);
        let diagnosis = sonic.inspect(&path)?;
        assert!(diagnosis.contains("Valid JSON"));

        cleanup();
        Ok(())
    }

    #[tokio::test]
    async fn test_repair_malformed_json() -> Result<()> {
        // PsychicPaper can handle markdown code blocks or messy JSON
        let (path, cleanup) = create_temp_file("```json\n{\"key\": \"value\"}\n```");

        let sonic = SonicScrewdriver::new(None, None, None);
        let report = sonic.repair(&path).await?;

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
    async fn test_diagnose_no_ai() -> Result<()> {
        let telemetry = Arc::new(TelemetryStore::new());
        let gallifrey = Arc::new(Gallifrey::new());
        // No Vortex provided
        let sonic = SonicScrewdriver::new(Some(telemetry), Some(gallifrey), None);

        let report = sonic.diagnose().await?;
        assert!(report.contains("SYSTEM DIAGNOSIS"));
        Ok(())
    }
}
