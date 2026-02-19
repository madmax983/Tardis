//! Time Capsule: Export/Import bi-temporal knowledge subgraphs.
//!
//! "Share your timeline! Send a piece of history to another Tardis."
//!
//! This module allows capturing a subgraph of the knowledge graph (entities and relationships)
//! starting from a root entity, and restoring it into another Gallifrey instance.

use crate::domain::{Entity, Relationship};
use crate::error::GallifreyResult;
use crate::traits::GallifreyService;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex};

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
    pub async fn capture(
        gallifrey: &dyn GallifreyService,
        root_name: &str,
        depth: usize,
    ) -> GallifreyResult<Self> {
        let mut capsule = Self::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        let root_name_owned = root_name.to_string();
        let root_id_found = Arc::new(Mutex::new(None));
        let root_id_clone = root_id_found.clone();

        // 1. Find the root entity
        gallifrey.scan_history(Box::new(move |history| {
            if let Ok(mut guard) = root_id_clone.lock() {
                if guard.is_some() {
                    return;
                }
                // Find the most current version of the entity with the matching name
                if let Some(entity) = history
                    .iter()
                    .find(|e| e.name == root_name_owned && e.temporal.is_current())
                {
                    *guard = Some(entity.id);
                }
            }
        }))?;

        let root_id = {
            let guard = root_id_found.lock().map_err(|e| {
                crate::error::GallifreyError::Internal(format!("Mutex error: {e}"))
            })?;
            guard.ok_or_else(|| {
                crate::error::GallifreyError::EntityNotFound(format!("Name: {root_name}"))
            })?
        };

        queue.push_back((root_id, 0));
        visited.insert(root_id);

        let mut visited_rels = HashSet::new();

        // 2. BFS Traversal
        while let Some((current_id, current_depth)) = queue.pop_front() {
            // Add entity history to capsule
            if let Ok(history) = gallifrey.get_history(current_id).await {
                capsule.entities.extend(history);
            }

            if current_depth >= depth {
                continue;
            }

            // Find relationships
            // We need to collect next_ids and relationships in a thread-safe way for the closure
            let captured_rels = Arc::new(Mutex::new(Vec::new()));
            let next_ids_found = Arc::new(Mutex::new(Vec::new()));
            let captured_rels_clone = captured_rels.clone();
            let next_ids_clone = next_ids_found.clone();
            // We need to pass the current set of visited nodes to avoid re-visiting?
            // No, the closure is just scanning. We process the results after.
            // But we need to know `visited_rels` to avoid duplicates.
            // But `visited_rels` is local. We can't access it in the closure easily if we want to mutate it.
            // Easier: collect all relevant rels, then filter in the main loop.

            gallifrey.scan_relationships(Box::new(move |rels| {
                if let Ok(mut rels_guard) = captured_rels_clone.lock() {
                    if let Ok(mut next_ids_guard) = next_ids_clone.lock() {
                        for rel in rels {
                            // Outgoing edges
                            if rel.source == current_id {
                                rels_guard.push(rel.clone());
                                next_ids_guard.push(rel.target);
                            }
                            // Incoming edges (optional, but good for context)
                            if rel.target == current_id {
                                rels_guard.push(rel.clone());
                                next_ids_guard.push(rel.source);
                            }
                        }
                    }
                }
            }))?;

            let rels_to_process = captured_rels.lock().map_err(|e| {
                crate::error::GallifreyError::Internal(format!("Mutex error: {e}"))
            })?;
            let ids_to_process = next_ids_found.lock().map_err(|e| {
                crate::error::GallifreyError::Internal(format!("Mutex error: {e}"))
            })?;

            for rel in rels_to_process.iter() {
                if visited_rels.insert(rel.id) {
                    capsule.relationships.push(rel.clone());
                }
            }

            for id in ids_to_process.iter() {
                if !visited.contains(id) && visited.insert(*id) {
                    queue.push_back((*id, current_depth + 1));
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
    pub async fn restore(&self, gallifrey: &dyn GallifreyService) -> GallifreyResult<()> {
        // Insert entities
        for entity in &self.entities {
            // We use the public insert method which handles locking
            let _ = gallifrey.insert(entity.clone()).await.map_err(|e| {
                crate::error::GallifreyError::StorageError(format!("Failed to insert entity: {e}"))
            })?;
        }

        // Insert relationships
        for rel in &self.relationships {
            gallifrey.insert_relationship(rel.clone())?;
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
        use crate::Gallifrey;
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
            .insert_relationship(rel)
            .unwrap();

        // Capture with depth 2 to ensure deduplication works (B will be processed)
        let capsule = TimeCapsule::capture(&source_gallifrey, "A", 2).await.unwrap();
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
