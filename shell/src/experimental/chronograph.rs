//! `ChronoGraph` 🗺️
//!
//! A visualizer for the bi-temporal knowledge graph.
//!
//! It traverses the graph starting from a root entity and generates
//! an ASCII representation of its relationships, respecting valid time.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fmt::Write;
use std::sync::Arc;
use tardis_common::id::EntityId;
use tardis_gallifrey::domain::Entity;
use tardis_gallifrey::GallifreyService;

/// The `ChronoGraph` visualizer.
#[derive(Debug)]
pub struct ChronoGraph {
    gallifrey: Arc<dyn GallifreyService>,
}

impl ChronoGraph {
    /// Create a new `ChronoGraph`.
    #[must_use]
    pub fn new(gallifrey: Arc<dyn GallifreyService>) -> Self {
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

    /// Generate a bi-temporal timeline of an entity.
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found.
    pub fn generate_timeline(&self, entity_name: &str) -> Result<String> {
        // Use find_entity_by_name with None to search current valid time,
        // but scan_history in find_entity_by_name will search everything if we modify it or use scan_history directly here.
        // Actually find_entity_by_name uses scan_history but filters.
        // We should just find the entity ID first.
        let root = self.find_entity_by_name(entity_name, None)?;

        let mut history = self
            .gallifrey
            .get_history(root.id)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        // Sort by valid time start (primary) and transaction time start (secondary)
        history.sort_by(|a, b| {
            a.temporal
                .valid_time
                .start
                .cmp(&b.temporal.valid_time.start)
                .then(
                    a.temporal
                        .transaction_time
                        .start
                        .cmp(&b.temporal.transaction_time.start),
                )
        });

        let mut timeline = String::new();
        writeln!(
            timeline,
            "⏳ Timeline: {} ({})",
            root.name, root.entity_type
        )?;
        writeln!(timeline, "========================================")?;

        for (i, version) in history.iter().enumerate() {
            writeln!(timeline, "\n[v{}]", i + 1)?;

            // Format Valid Time
            let vt_start = version
                .temporal
                .valid_time
                .start
                .format("%Y-%m-%d %H:%M:%S");
            let vt_end = match version.temporal.valid_time.end {
                Some(t) => t.format("%Y-%m-%d %H:%M:%S").to_string(),
                None => "FOREVER".to_string(),
            };
            writeln!(timeline, "  Valid:        {vt_start} -> {vt_end}")?;

            // Format Transaction Time
            let tt_start = version
                .temporal
                .transaction_time
                .start
                .format("%Y-%m-%d %H:%M:%S");
            let tt_end = match version.temporal.transaction_time.end {
                Some(t) => t.format("%Y-%m-%d %H:%M:%S").to_string(),
                None => "CURRENT".to_string(),
            };
            writeln!(timeline, "  Transaction:  {tt_start} -> {tt_end}")?;

            // Show properties
            if !version.properties.is_empty() {
                // Pretty print properties
                let props = serde_json::to_string_pretty(&version.properties)
                    .unwrap_or_else(|_| format!("{:?}", version.properties));
                // Indent properties
                let indented_props = props.replace('\n', "\n    ");
                writeln!(timeline, "  Properties:   {indented_props}")?;
            }
        }

        Ok(timeline)
    }

    fn find_entity_by_name(&self, name: &str, time: Option<DateTime<Utc>>) -> Result<Entity> {
        let found = Arc::new(std::sync::Mutex::new(None));
        let found_clone = found.clone();
        let name_owned = name.to_string();

        // Scan all entities to find the one with the matching name
        self.gallifrey
            .scan_history(Box::new(move |history| {
                if let Ok(mut guard) = found_clone.lock() {
                    if guard.is_some() {
                        return;
                    }

                    let entity = if let Some(t) = time {
                        history.iter().find(|e| {
                            e.name == name_owned && e.temporal.valid_time.contains(t)
                        })
                    } else {
                        history
                            .iter()
                            .find(|e| e.name == name_owned && e.temporal.is_current())
                    };

                    if let Some(e) = entity {
                        *guard = Some(e.clone());
                    }
                }
            }))
            .map_err(|e| anyhow!(e.to_string()))?;

        let res = {
            let mut guard = found.lock().map_err(|e| anyhow!("Mutex error: {e}"))?;
            guard.take()
        };
        res.ok_or_else(|| anyhow!("Entity '{name}' not found"))
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
        let relevant = Arc::new(std::sync::Mutex::new(Vec::new()));
        let relevant_clone = relevant.clone();

        self.gallifrey
            .scan_relationships(Box::new(move |rels| {
                if let Ok(mut guard) = relevant_clone.lock() {
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
                            guard.push(rel.clone());
                        }
                    }
                }
            }))
            .map_err(|e| anyhow!(e.to_string()))?;

        let res = relevant
            .lock()
            .map_err(|e| anyhow!("Mutex error: {e}"))?;
        Ok(res.clone())
    }

    fn resolve_entity(&self, id: EntityId, time: Option<DateTime<Utc>>) -> Result<Option<Entity>> {
        if let Some(t) = time {
            self.gallifrey
                .get_entity_at(id, t, Utc::now())
                .map_err(|e| anyhow!(e.to_string()))
        } else {
            // If no time specified, get current. But Gallifrey facade doesn't expose get_entity(id) directly
            // except via get_history (then pick last) or get_entity_at(now).
            // `get_history` is overkill.
            // But wait, `get_entity_at` with `now` is correct for current view.
            self.gallifrey
                .get_entity_at(id, Utc::now(), Utc::now())
                .map_err(|e| anyhow!(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration;
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
        use tardis_gallifrey::Gallifrey;
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
        println!("{}", map);

        assert!(map.contains("A (Test)"));
        assert!(map.contains("LINKS_TO"));
        assert!(map.contains("B (Test)"));
    }

    #[tokio::test]
    async fn test_chronograph_cycle() {
        use tardis_gallifrey::Gallifrey;
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

        gallifrey.insert_relationship(rel1).unwrap();
        gallifrey.insert_relationship(rel2).unwrap();

        let map = graph.generate_map("A", 3, None).unwrap();
        println!("{}", map);

        // Should contain A, B, and a cycle marker
        assert!(map.contains("A (Test)"));
        assert!(map.contains("B (Test)"));
        assert!(map.contains("🔄"));
    }

    #[tokio::test]
    async fn test_chronograph_timeline() {
        use tardis_gallifrey::Gallifrey;
        let gallifrey = Arc::new(Gallifrey::new());
        let graph = ChronoGraph::new(gallifrey.clone());

        let id = EntityId::new();
        let mut entity = create_test_entity("Timelord", id);
        entity
            .properties
            .insert("regeneration".to_string(), json!(1));
        gallifrey.insert(entity).await.unwrap();

        // Sleep to ensure measurable time difference
        thread::sleep(Duration::from_millis(10));

        // Update
        gallifrey
            .update(id, json!({"regeneration": 2}))
            .await
            .unwrap();

        let timeline = graph.generate_timeline("Timelord").unwrap();
        println!("{}", timeline);

        assert!(timeline.contains("[v1]"));
        assert!(timeline.contains("[v2]"));
        assert!(timeline.contains("regeneration"));
        assert!(timeline.contains("1"));
        assert!(timeline.contains("2"));
    }
}
