//! ChronoGraph 🗺️
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
use tardis_gallifrey::Gallifrey;

/// The ChronoGraph visualizer.
#[derive(Debug)]
pub struct ChronoGraph {
    gallifrey: Arc<Gallifrey>,
}

impl ChronoGraph {
    /// Create a new ChronoGraph.
    #[must_use]
    pub fn new(gallifrey: Arc<Gallifrey>) -> Self {
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

        writeln!(map, "🗺️  Time Map: {} (Depth: {})", entity_name, depth)?;
        if let Some(t) = time {
            writeln!(map, "🕒  Time: {}", t)?;
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

        found.ok_or_else(|| anyhow!("Entity '{}' not found", name))
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
            writeln!(
                output,
                "{}|--[{}]-->",
                indent_rel, rel.relationship_type
            )?;

            if let Some(target) = self.resolve_entity(rel.target, time)? {
                self.traverse(
                    &target,
                    max_depth,
                    current_depth + 1,
                    time,
                    visited,
                    output,
                )?;
            } else {
                writeln!(output, "{}(Unknown Entity)", indent_rel)?;
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

    fn resolve_entity(&self, id: EntityId, time: Option<DateTime<Utc>>) -> Result<Option<Entity>> {
        let knowledge = self.gallifrey.knowledge();
        if let Some(t) = time {
            knowledge
                .get_entity_at(id, t, Utc::now())
                .map_err(|e| anyhow!(e.to_string()))
        } else {
            knowledge
                .get_entity(id)
                .map_err(|e| anyhow!(e.to_string()))
        }
    }
}

#[cfg(test)]
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
        println!("{}", map);

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
         println!("{}", map);

         // Should contain A, B, and a cycle marker
         assert!(map.contains("A (Test)"));
         assert!(map.contains("B (Test)"));
         assert!(map.contains("🔄"));
    }
}
