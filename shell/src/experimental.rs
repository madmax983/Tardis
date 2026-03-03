//! Experimental features for the Tardis Shell.
//!
//! Includes:
//! - **ChronoGraph**: ASCII visualization of the temporal knowledge graph.
//! - **Sonic Screwdriver**: System diagnosis and file repair tool.

// Re-export features
pub use chronograph::ChronoGraph;
pub use sonic::SonicScrewdriver;

/// `ChronoGraph` 🗺️
///
/// A visualizer for the bi-temporal knowledge graph.
///
/// It traverses the graph starting from a root entity and generates
/// an ASCII representation of its relationships, respecting valid time.
pub mod chronograph {
    use anyhow::{anyhow, Result};
    use chrono::{DateTime, Utc};
    use std::collections::HashSet;
    use std::fmt::Write;
    use std::sync::Arc;
    use tardis_common::id::EntityId;
    use tardis_gallifrey::domain::Entity;
    use tardis_gallifrey::Gallifrey;

    /// The `ChronoGraph` visualizer.
    #[derive(Debug)]
    pub struct ChronoGraph {
        gallifrey: Arc<Gallifrey>,
    }

    impl ChronoGraph {
        /// Create a new `ChronoGraph`.
        #[must_use]
        pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
            Self { gallifrey }
        }

        /// Generate an ASCII map of the knowledge graph around an entity.
        ///
        /// # Errors
        ///
        /// Returns an error if the entity is not found or graph traversal fails.
        pub fn generate_map(
            &self,
            entity_name: &str,
            depth: usize,
            time: Option<DateTime<Utc>>,
        ) -> Result<String> {
            let root = self.find_entity_by_name(entity_name, time)?;
            let mut visited = HashSet::new();
            let mut map = String::new();

            writeln!(map, "🗺️  Time Map: {entity_name} (Depth: {depth})")?;
            if let Some(t) = time {
                writeln!(map, "🕒  Time: {t}")?;
            } else {
                writeln!(map, "🕒  Time: NOW (Current Valid)")?;
            }
            writeln!(map, "========================================\n")?;

            self.traverse(&root, depth, 0, time, &mut visited, &mut map)?;

            Ok(map)
        }

        fn find_entity_by_name(&self, name: &str, time: Option<DateTime<Utc>>) -> Result<Entity> {
            let knowledge = self.gallifrey.knowledge();
            let mut found = None;

            // Scan all entities to find the one with the matching name
            knowledge.scan_history(|history| {
                if found.is_some() {
                    return;
                }

                let entity = if let Some(t) = time {
                    history
                        .iter()
                        .find(|e| e.name == name && e.temporal.valid_time.contains(t))
                } else {
                    history
                        .iter()
                        .find(|e| e.name == name && e.temporal.is_current())
                };

                if let Some(e) = entity {
                    found = Some(e.clone());
                }
            })?;

            found.ok_or_else(|| anyhow!("Entity '{name}' not found"))
        }

        fn traverse(
            &self,
            entity: &Entity,
            max_depth: usize,
            current_depth: usize,
            time: Option<DateTime<Utc>>,
            visited: &mut HashSet<EntityId>,
            output: &mut String,
        ) -> Result<()> {
            let indent = "  ".repeat(current_depth);
            let prefix = if current_depth == 0 { "" } else { "└── " };

            // Print the node
            write!(
                output,
                "{}{}{} ({})",
                indent, prefix, entity.name, entity.entity_type
            )?;

            // Cycle detection / Already visited in this traversal
            if visited.contains(&entity.id) {
                writeln!(output, " 🔄")?;
                return Ok(());
            }
            writeln!(output)?;
            visited.insert(entity.id);

            if current_depth >= max_depth {
                return Ok(());
            }

            // Find outgoing relationships
            let relationships = self.find_relationships(entity.id, time)?;

            for rel in relationships {
                // Only follow outgoing edges
                if rel.source != entity.id {
                    continue;
                }

                let indent_rel = "  ".repeat(current_depth + 1);
                writeln!(output, "{}|--[{}]-->", indent_rel, rel.relationship_type)?;

                if let Some(target) = self.resolve_entity(rel.target, time)? {
                    self.traverse(&target, max_depth, current_depth + 1, time, visited, output)?;
                } else {
                    writeln!(output, "{indent_rel}(Unknown Entity)")?;
                }
            }

            Ok(())
        }

        fn find_relationships(
            &self,
            entity_id: EntityId,
            time: Option<DateTime<Utc>>,
        ) -> Result<Vec<tardis_gallifrey::domain::Relationship>> {
            let knowledge = self.gallifrey.knowledge();
            let mut relevant = Vec::new();

            knowledge.scan_relationships(|rels| {
                for rel in rels {
                    if rel.source != entity_id {
                        continue;
                    }

                    let is_valid = if let Some(t) = time {
                        rel.temporal.valid_time.contains(t)
                    } else {
                        rel.temporal.is_current()
                    };

                    if is_valid {
                        relevant.push(rel.clone());
                    }
                }
            })?;

            Ok(relevant)
        }

        fn resolve_entity(
            &self,
            id: EntityId,
            time: Option<DateTime<Utc>>,
        ) -> Result<Option<Entity>> {
            let knowledge = self.gallifrey.knowledge();
            if let Some(t) = time {
                knowledge
                    .get_entity_at(id, t, Utc::now())
                    .map_err(|e| anyhow!(e.to_string()))
            } else {
                knowledge.get_entity(id).map_err(|e| anyhow!(e.to_string()))
            }
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    mod tests {
        use super::*;
        use std::collections::HashMap;
        use tardis_common::temporal::BiTemporalInterval;
        use tardis_gallifrey::domain::Relationship;

        fn create_test_entity(name: &str, id: EntityId) -> Entity {
            Entity {
                id,
                entity_type: "Test".to_string(),
                name: name.to_string(),
                properties: HashMap::new(),
                embedding: None,
                temporal: BiTemporalInterval::now(),
                source: None,
            }
        }

        #[tokio::test]
        async fn test_chronograph_basic() {
            let gallifrey = Arc::new(Gallifrey::new());
            let graph = ChronoGraph::new(gallifrey.clone());

            let id_a = EntityId::new();
            let id_b = EntityId::new();

            let entity_a = create_test_entity("A", id_a);
            let entity_b = create_test_entity("B", id_b);

            gallifrey.insert(entity_a).await.unwrap();
            gallifrey.insert(entity_b).await.unwrap();

            // Create relationship A -> B
            let rel = Relationship {
                id: EntityId::new(),
                relationship_type: "LINKS_TO".to_string(),
                source: id_a,
                target: id_b,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            };

            gallifrey.knowledge().insert_relationship(rel).unwrap();

            let map = graph.generate_map("A", 2, None).unwrap();
            println!("{map}");

            assert!(map.contains("A (Test)"));
            assert!(map.contains("LINKS_TO"));
            assert!(map.contains("B (Test)"));
        }

        #[tokio::test]
        async fn test_chronograph_cycle() {
            let gallifrey = Arc::new(Gallifrey::new());
            let graph = ChronoGraph::new(gallifrey.clone());

            let id_a = EntityId::new();
            let id_b = EntityId::new();

            let entity_a = create_test_entity("A", id_a);
            let entity_b = create_test_entity("B", id_b);

            gallifrey.insert(entity_a).await.unwrap();
            gallifrey.insert(entity_b).await.unwrap();

            // A -> B
            let rel1 = Relationship {
                id: EntityId::new(),
                relationship_type: "TO".to_string(),
                source: id_a,
                target: id_b,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            };
            // B -> A
            let rel2 = Relationship {
                id: EntityId::new(),
                relationship_type: "BACK".to_string(),
                source: id_b,
                target: id_a,
                properties: HashMap::new(),
                temporal: BiTemporalInterval::now(),
            };

            gallifrey.knowledge().insert_relationship(rel1).unwrap();
            gallifrey.knowledge().insert_relationship(rel2).unwrap();

            let map = graph.generate_map("A", 3, None).unwrap();
            println!("{map}");

            // Should contain A, B, and a cycle marker
            assert!(map.contains("A (Test)"));
            assert!(map.contains("B (Test)"));
            assert!(map.contains("🔄"));
        }
    }
}

/// The Sonic Screwdriver 🛠️
///
/// "It's a scientific instrument, not a magic wand!"
///
/// A CLI tool for file repair and analysis, leveraging `PsychicPaper`
/// for heuristic parsing and validation.
pub mod sonic {
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
                                    diagnosis
                                        .push_str("Type: Unknown / Text\nStatus: Ambiguous 🟡");
                                } else {
                                    diagnosis.push_str(
                                        "Type: Structured (Auto-detected)\nStatus: Healthy 🟢",
                                    );
                                }
                            }
                            Err(e) => {
                                let _ = write!(
                                    diagnosis,
                                    "Type: Unknown\nStatus: Broken 🔴\nError: {e}"
                                );
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
    #[allow(clippy::unwrap_used)]
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
            let path = std::env::temp_dir().join(format!("sonic_test_{nanos}.tmp"));
            let mut file = File::create(&path).unwrap();
            write!(file, "{content}").unwrap();

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
    }
}
