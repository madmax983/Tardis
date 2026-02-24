//! The Codex 📜
//!
//! An exporter for the Knowledge Graph.
//!
//! Converts the internal graph representation into a set of interlinked
//! Markdown files, suitable for tools like Obsidian or for static site generation.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tardis_common::domain::{Entity, Relationship};
use tardis_common::id::EntityId;
use tardis_gallifrey::Gallifrey;

/// The Codex exporter.
#[derive(Debug)]
pub struct Codex {
    gallifrey: Arc<Gallifrey>,
}

impl Codex {
    /// Create a new Codex.
    #[must_use]
    pub const fn new(gallifrey: Arc<Gallifrey>) -> Self {
        Self { gallifrey }
    }

    /// Export the Knowledge Graph to the specified directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or files cannot be written.
    pub fn export(&self, output_dir: &Path) -> Result<String> {
        fs::create_dir_all(output_dir).context("Failed to create output directory")?;

        let mut entities: HashMap<EntityId, Entity> = HashMap::new();
        let mut relationships: HashMap<EntityId, Vec<Relationship>> = HashMap::new();

        // 1. Scan for latest entities
        self.gallifrey.knowledge().scan_history(|history| {
            if let Some(latest) = history.last() {
                entities.insert(latest.id, latest.clone());
            }
        })?;

        // 2. Scan relationships
        self.gallifrey.knowledge().scan_relationships(|rels| {
            for rel in rels {
                relationships
                    .entry(rel.source)
                    .or_default()
                    .push(rel.clone());
            }
        })?;

        // 3. Generate files
        let mut count = 0;
        for entity in entities.values() {
            let filename = Self::sanitize_filename(&entity.name);
            let path = output_dir.join(format!("{filename}.md"));

            let rels = relationships
                .get(&entity.id)
                .map_or(&[] as &[Relationship], std::vec::Vec::as_slice);

            let content = Self::generate_markdown(entity, rels, &entities);

            fs::write(&path, content)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            count += 1;
        }

        // 4. Generate Index
        let index_content = Self::generate_index(&entities);
        fs::write(output_dir.join("index.md"), index_content)?;

        Ok(format!(
            "Exported {count} entities to {}",
            output_dir.display()
        ))
    }

    fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn generate_markdown(
        entity: &Entity,
        rels: &[Relationship],
        lookup: &HashMap<EntityId, Entity>,
    ) -> String {
        let mut md = String::new();

        // Frontmatter (YAML-like)
        let _ = writeln!(md, "---");
        let _ = writeln!(md, "type: {}", entity.entity_type);
        let _ = writeln!(md, "id: {}", entity.id);
        let _ = writeln!(md, "valid_from: {:?}", entity.temporal.valid_time);
        let _ = writeln!(md, "---\n");

        // Title
        let _ = writeln!(md, "# {}\n", entity.name);

        // Properties
        if !entity.properties.is_empty() {
            let _ = writeln!(md, "## Properties\n");
            let _ = writeln!(md, "| Property | Value |");
            let _ = writeln!(md, "| --- | --- |");
            for (k, v) in &entity.properties {
                let val_str = if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    v.to_string()
                };
                let _ = writeln!(md, "| {k} | {val_str} |");
            }
            md.push('\n');
        }

        // Relationships
        if !rels.is_empty() {
            let _ = writeln!(md, "## Relationships\n");
            for rel in rels {
                let target_name = lookup
                    .get(&rel.target)
                    .map_or("Unknown Entity", |e| e.name.as_str());

                // Obsidian style link
                let _ = writeln!(md, "- **{}** -> [[{}]]", rel.relationship_type, target_name);
            }
            md.push('\n');
        }

        // Embedding (Optional visualization or just note)
        if entity.embedding.is_some() {
            let _ = writeln!(md, "## Metadata\n");
            let _ = writeln!(md, "*Contains vector embedding*");
        }

        md
    }

    fn generate_index(entities: &HashMap<EntityId, Entity>) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "# Knowledge Graph Index\n");

        // Sort by name
        let mut sorted: Vec<&Entity> = entities.values().collect();
        sorted.sort_by_key(|e| &e.name);

        for entity in sorted {
            let _ = writeln!(md, "- [[{}]] ({})", entity.name, entity.entity_type);
        }
        md
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tardis_common::temporal::BiTemporalInterval;

    #[test]
    fn test_sanitize() {
        assert_eq!(Codex::sanitize_filename("Hello World"), "Hello_World");
        assert_eq!(Codex::sanitize_filename("foo/bar"), "foo_bar");
    }

    #[test]
    fn test_markdown_generation() {
        let mut props = HashMap::new();
        props.insert("role".to_string(), serde_json::json!("Protagonist"));

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "Person".to_string(),
            name: "The Doctor".to_string(),
            properties: props,
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        };

        let lookup = HashMap::from([(entity.id, entity.clone())]);
        let md = Codex::generate_markdown(&entity, &[], &lookup);

        assert!(md.contains("# The Doctor"));
        assert!(md.contains("type: Person"));
        assert!(md.contains("| role | Protagonist |"));
    }
}
