//! Time Capsule: Export/Import bi-temporal knowledge subgraphs.
//!
//! "Share your timeline! Send a piece of history to another Tardis."
//!
//! This module allows capturing a subgraph of the knowledge graph (entities and relationships)
//! starting from a root entity, and restoring it into another Gallifrey instance.

use crate::domain::{Entity, Relationship};
use crate::error::GallifreyResult;
use crate::Gallifrey;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// A portable container for a knowledge subgraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCapsule {
    /// The captured entities.
    pub entities: Vec<Entity>,
    /// The captured relationships.
    pub relationships: Vec<Relationship>,
}

impl TimeCapsule {
    /// Create a new empty `TimeCapsule`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Capture a subgraph starting from a root entity.
    ///
    /// Performs a Breadth-First Search (BFS) to find all reachable entities and relationships
    /// up to the specified depth.
    ///
    /// # Errors
    ///
    /// Returns an error if the root entity is not found or if graph traversal fails.
    pub fn capture(gallifrey: &Gallifrey, root_name: &str, depth: usize) -> GallifreyResult<Self> {
        let knowledge = gallifrey.knowledge();
        let mut capsule = Self::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // 1. Find the root entity
        let mut root_id = None;
        knowledge.scan_history(|history| {
            // Find the most current version of the entity with the matching name
            if let Some(entity) = history
                .iter()
                .find(|e| e.name == root_name && e.temporal.is_current())
            {
                root_id = Some(entity.id);
            }
        })?;

        let root_id = root_id.ok_or_else(|| {
            crate::error::GallifreyError::EntityNotFound(format!("Name: {root_name}"))
        })?;

        queue.push_back((root_id, 0));
        visited.insert(root_id);

        let mut visited_rels = HashSet::new();

        // 2. BFS Traversal
        while let Some((current_id, current_depth)) = queue.pop_front() {
            // Add entity history to capsule
            if let Ok(history) = knowledge.get_entity_history(current_id) {
                capsule.entities.extend(history);
            }

            if current_depth >= depth {
                continue;
            }

            // Find relationships
            let mut next_ids = Vec::new();
            knowledge.scan_relationships(|rels| {
                for rel in rels {
                    // Outgoing edges
                    if rel.source == current_id {
                        if visited_rels.insert(rel.id) {
                            capsule.relationships.push(rel.clone());
                        }
                        if !visited.contains(&rel.target) {
                            next_ids.push(rel.target);
                        }
                    }
                    // Incoming edges (optional, but good for context)
                    if rel.target == current_id {
                        if visited_rels.insert(rel.id) {
                            capsule.relationships.push(rel.clone());
                        }
                        if !visited.contains(&rel.source) {
                            next_ids.push(rel.source);
                        }
                    }
                }
            })?;

            for id in next_ids {
                if visited.insert(id) {
                    queue.push_back((id, current_depth + 1));
                }
            }
        }

        Ok(capsule)
    }

    /// Restore the capsule into a Gallifrey instance.
    ///
    /// This merges the captured entities and relationships into the target knowledge graph.
    /// - Entities are inserted (append-only, so new versions or new entities).
    /// - Relationships are inserted.
    ///
    /// # Errors
    ///
    /// Returns an error if insertion fails.
    pub async fn restore(&self, gallifrey: &Gallifrey) -> GallifreyResult<()> {
        // Insert entities
        for entity in &self.entities {
            // We use the public insert method which handles locking
            // Note: In a real system, we might want to check for conflicts or merge strategies.
            // Here, we trust Gallifrey's append-only nature.
            // However, inserting an entity with an existing ID will create a new version.
            // If the entity is historical (not current), we might need special handling if we want to preserve exact history.
            // But Gallifrey::insert creates a new version with *current* transaction time.
            // This means restored history becomes "new knowledge about the past".
            // This is actually correct for a bitemporal system receiving data!
            let _ = gallifrey.insert(entity.clone()).await.map_err(|e| {
                crate::error::GallifreyError::StorageError(format!("Failed to insert entity: {e}"))
            })?;
        }

        // Insert relationships
        let knowledge = gallifrey.knowledge();
        for rel in &self.relationships {
            knowledge.insert_relationship(rel.clone())?;
        }

        Ok(())
    }

    /// Save the capsule to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if file creation or serialization fails.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> GallifreyResult<()> {
        let file = File::create(path).map_err(|e| {
            crate::error::GallifreyError::StorageError(format!("Failed to create file: {e}"))
        })?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).map_err(|e| {
            crate::error::GallifreyError::StorageError(format!("Failed to serialize capsule: {e}"))
        })?;
        Ok(())
    }

    /// Load a capsule from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or deserialization fails.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> GallifreyResult<Self> {
        let file = File::open(path).map_err(|e| {
            crate::error::GallifreyError::StorageError(format!("Failed to open file: {e}"))
        })?;
        let reader = BufReader::new(file);
        let capsule = serde_json::from_reader(reader).map_err(|e| {
            crate::error::GallifreyError::StorageError(format!(
                "Failed to deserialize capsule: {e}"
            ))
        })?;
        Ok(capsule)
    }
}

impl Default for TimeCapsule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gallifrey;
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: "Test".to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: None,
        }
    }

    #[tokio::test]
    async fn test_capture_restore() {
        let source_gallifrey = Gallifrey::new();
        let target_gallifrey = Gallifrey::new();

        let entity_a = create_test_entity("A");
        let entity_b = create_test_entity("B");
        let id_a = entity_a.id;
        let id_b = entity_b.id;

        source_gallifrey.insert(entity_a).await.unwrap();
        source_gallifrey.insert(entity_b).await.unwrap();

        // Create relationship A -> B
        let rel = Relationship {
            id: EntityId::new(),
            relationship_type: "LINKS_TO".to_string(),
            source: id_a,
            target: id_b,
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };
        source_gallifrey
            .knowledge()
            .insert_relationship(rel)
            .unwrap();

        // Capture with depth 2 to ensure deduplication works (B will be processed)
        let capsule = TimeCapsule::capture(&source_gallifrey, "A", 2).unwrap();
        assert_eq!(capsule.entities.len(), 2); // A and B
        assert_eq!(capsule.relationships.len(), 1); // Should be 1 (deduplicated)

        // Save and Load (test serialization)
        let temp_path = std::env::temp_dir().join("test_capsule.json");
        capsule.save_to_file(&temp_path).unwrap();
        let loaded_capsule = TimeCapsule::load_from_file(&temp_path).unwrap();

        // Restore
        loaded_capsule.restore(&target_gallifrey).await.unwrap();

        // Verify restoration
        let restored_a = target_gallifrey.knowledge().get_entity(id_a).unwrap();
        assert!(restored_a.is_some());
        assert_eq!(restored_a.unwrap().name, "A");

        let restored_b = target_gallifrey.knowledge().get_entity(id_b).unwrap();
        assert!(restored_b.is_some());
        assert_eq!(restored_b.unwrap().name, "B");

        // Clean up
        let _ = std::fs::remove_file(temp_path);
    }
}
