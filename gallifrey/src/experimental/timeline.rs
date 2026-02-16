//! Timeline simulation for "What-If" scenarios.
//!
//! This module provides a `Timeline` struct that acts as a copy-on-write overlay
//! on top of a base `Gallifrey` instance. It allows experimenting with state changes
//! (inserts, updates, deletes) without affecting the main timeline.

use crate::domain::{Entity, Relationship};
use crate::error::{GallifreyError, GallifreyResult};
use crate::stores::KnowledgeStore;
use crate::Gallifrey;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tardis_common::id::EntityId;

/// A divergent timeline that overlays changes on top of a base Gallifrey instance.
///
/// This allows "what-if" simulations without affecting the main timeline.
/// Changes are stored locally in `overlay` and `deleted`.
/// Reads check local state first, then fall back to `base`.
#[derive(Debug)]
pub struct Timeline {
    base: Arc<Gallifrey>,
    overlay: Arc<KnowledgeStore>,
    deleted_entities: RwLock<HashSet<EntityId>>,
    deleted_relationships: RwLock<HashSet<EntityId>>, // Relationship IDs are also EntityId
}

impl Timeline {
    /// Create a new timeline branching off the given base.
    #[must_use]
    pub fn new(base: Arc<Gallifrey>) -> Self {
        Self {
            base,
            overlay: Arc::new(KnowledgeStore::new()),
            deleted_entities: RwLock::new(HashSet::new()),
            deleted_relationships: RwLock::new(HashSet::new()),
        }
    }

    /// Get an entity from the timeline.
    ///
    /// Checks overlay first, then deleted set, then base.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_entity(&self, id: EntityId) -> GallifreyResult<Option<Entity>> {
        // Check if deleted locally
        if self
            .deleted_entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .contains(&id)
        {
            return Ok(None);
        }

        // Check overlay
        if let Some(entity) = self.overlay.get_entity(id)? {
            return Ok(Some(entity));
        }

        // Fallback to base
        self.base.knowledge().get_entity(id)
    }

    /// Insert an entity into the timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn insert_entity(&self, entity: Entity) -> GallifreyResult<EntityId> {
        let id = entity.id;

        // Remove from deleted set if it was there (undelete)
        self.deleted_entities
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .remove(&id);

        self.overlay.insert_entity(entity)
    }

    /// Update an entity in the timeline.
    ///
    /// If the entity exists in base but not overlay, it is copied to overlay first (COW).
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or lock is poisoned.
    pub fn update_entity(
        &self,
        id: EntityId,
        updates: HashMap<String, serde_json::Value>,
    ) -> GallifreyResult<()> {
        // Check if deleted
        if self
            .deleted_entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .contains(&id)
        {
            return Err(GallifreyError::EntityNotFound(id.to_string()));
        }

        // Check if exists in overlay
        if self.overlay.get_entity(id)?.is_none() {
            // Not in overlay, check base
            if let Some(base_entity) = self.base.knowledge().get_entity(id)? {
                // Copy to overlay
                self.overlay.insert_entity(base_entity)?;
            } else {
                return Err(GallifreyError::EntityNotFound(id.to_string()));
            }
        }

        // Now update in overlay
        self.overlay.update_entity(id, updates)
    }

    /// Delete an entity from the timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn delete_entity(&self, id: EntityId) -> GallifreyResult<()> {
        self.deleted_entities
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .insert(id);
        Ok(())
    }

    /// Insert a relationship into the timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn insert_relationship(&self, relationship: Relationship) -> GallifreyResult<EntityId> {
        let id = relationship.id;

        // Remove from deleted set if it was there (undelete)
        self.deleted_relationships
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .remove(&id);

        self.overlay.insert_relationship(relationship)
    }

    /// Delete a relationship from the timeline.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn delete_relationship(&self, id: EntityId) -> GallifreyResult<()> {
        self.deleted_relationships
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .insert(id);
        Ok(())
    }

    /// List changes in this timeline compared to the base.
    ///
    /// Returns (`modified_entities`, `deleted_entity_ids`).
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails.
    pub fn list_changes(&self) -> GallifreyResult<(Vec<Entity>, Vec<EntityId>)> {
        let mut modified = Vec::new();

        self.overlay.scan_history(|history| {
            // In the overlay, the history might contain multiple versions.
            // We are interested in the *latest* version which represents the current state in this timeline.
            if let Some(latest) = history.iter().find(|e| e.temporal.is_current()) {
                modified.push(latest.clone());
            }
        })?;

        let deleted = self
            .deleted_entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .iter()
            .copied()
            .collect();

        Ok((modified, deleted))
    }

    /// Scan history of all entities in the timeline (merging base and overlay).
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails.
    pub fn scan_history<F>(&self, mut visitor: F) -> GallifreyResult<()>
    where
        F: FnMut(&[Entity]),
    {
        let deleted = self
            .deleted_entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;
        let mut visited_ids = HashSet::new();

        // 1. Visit overlay (highest priority)
        self.overlay.scan_history(|history| {
            if let Some(first) = history.first() {
                if !deleted.contains(&first.id) {
                    visited_ids.insert(first.id);
                    visitor(history);
                }
            }
        })?;

        // 2. Visit base (if not visited and not deleted)
        self.base.knowledge().scan_history(|history| {
            if let Some(first) = history.first() {
                if !visited_ids.contains(&first.id) && !deleted.contains(&first.id) {
                    visitor(history);
                }
            }
        })?;

        Ok(())
    }

    /// Scan all relationships in the timeline (merging base and overlay).
    ///
    /// # Errors
    ///
    /// Returns an error if scanning fails.
    pub fn scan_relationships<F>(&self, mut visitor: F) -> GallifreyResult<()>
    where
        F: FnMut(&[Relationship]),
    {
        let deleted = self
            .deleted_relationships
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;
        let mut visited_ids = HashSet::new();

        // 1. Visit overlay
        self.overlay.scan_relationships(|rels| {
            // Note: scan_relationships yields Vec<Relationship> which might be versions of one relationship
            // or relationships for an entity?
            // Checking KnowledgeStore: relationships is HashMap<EntityId, Vec<Relationship>>
            // It seems key is relationship ID? No, wait.
            // KnowledgeStore: relationships: RwLock<HashMap<EntityId, Vec<Relationship>>>.
            // insert_relationship uses relationship.id as key.
            // So each entry is history of one relationship.

            if let Some(first) = rels.first() {
                if !deleted.contains(&first.id) {
                    visited_ids.insert(first.id);
                    visitor(rels);
                }
            }
        })?;

        // 2. Visit base
        self.base.knowledge().scan_relationships(|rels| {
            if let Some(first) = rels.first() {
                if !visited_ids.contains(&first.id) && !deleted.contains(&first.id) {
                    visitor(rels);
                }
            }
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::{Entity, Relationship};
    use std::collections::HashMap;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_entity(name: &str) -> Entity {
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
    async fn test_timeline_isolation() {
        let base = Arc::new(Gallifrey::new());
        let entity = create_entity("Base Entity");
        let id = entity.id;
        base.insert(entity.clone()).await.unwrap();

        let timeline = Timeline::new(base.clone());

        // Read from base via timeline
        let retrieved = timeline.get_entity(id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Base Entity");

        // Update in timeline
        let mut updates = HashMap::new();
        updates.insert("status".to_string(), serde_json::json!("Modified"));
        timeline.update_entity(id, updates).unwrap();

        // Verify timeline has update
        let modified = timeline.get_entity(id).unwrap().unwrap();
        assert_eq!(
            modified.properties.get("status"),
            Some(&serde_json::json!("Modified"))
        );

        // Verify base is unchanged
        let base_entity = base.knowledge().get_entity(id).unwrap().unwrap();
        assert!(base_entity.properties.is_empty());
    }

    #[tokio::test]
    async fn test_timeline_deletion() {
        let base = Arc::new(Gallifrey::new());
        let entity = create_entity("To Delete");
        let id = entity.id;
        base.insert(entity).await.unwrap();

        let timeline = Timeline::new(base.clone());

        timeline.delete_entity(id).unwrap();

        assert!(timeline.get_entity(id).unwrap().is_none());
        assert!(base.knowledge().get_entity(id).unwrap().is_some());
    }

    #[tokio::test]
    async fn test_scan_history_merge() {
        let base = Arc::new(Gallifrey::new());
        let e1 = create_entity("E1");
        let e2 = create_entity("E2");
        let e3 = create_entity("E3"); // Will be deleted in timeline
        let id1 = e1.id;
        let id2 = e2.id;
        let id3 = e3.id;

        base.insert(e1).await.unwrap();
        base.insert(e2).await.unwrap();
        base.insert(e3).await.unwrap();

        let timeline = Timeline::new(base.clone());

        // Modify E2
        let mut updates = HashMap::new();
        updates.insert("mod".to_string(), serde_json::json!(true));
        timeline.update_entity(id2, updates).unwrap();

        // Delete E3
        timeline.delete_entity(id3).unwrap();

        // Insert E4
        let e4 = create_entity("E4");
        let id4 = e4.id;
        timeline.insert_entity(e4).unwrap();

        let mut collected = HashMap::new();
        timeline
            .scan_history(|history| {
                let current = history.iter().find(|e| e.temporal.is_current()).unwrap();
                collected.insert(current.id, current.clone());
            })
            .unwrap();

        assert!(collected.contains_key(&id1)); // From base
        assert!(collected.contains_key(&id2)); // From overlay (modified)
        assert!(!collected.contains_key(&id3)); // Deleted
        assert!(collected.contains_key(&id4)); // New in overlay

        assert!(collected.get(&id2).unwrap().properties.contains_key("mod"));
    }

    #[tokio::test]
    async fn test_relationships_cow() {
        let base = Arc::new(Gallifrey::new());
        let timeline = Timeline::new(base.clone());

        let r1 = Relationship {
            id: EntityId::new(),
            relationship_type: "TEST".to_string(),
            source: EntityId::new(),
            target: EntityId::new(),
            properties: HashMap::new(),
            temporal: BiTemporalInterval::now(),
        };

        timeline.insert_relationship(r1.clone()).unwrap();

        let mut count = 0;
        timeline.scan_relationships(|_| count += 1).unwrap();
        assert_eq!(count, 1);

        timeline.delete_relationship(r1.id).unwrap();

        let mut count_after = 0;
        timeline.scan_relationships(|_| count_after += 1).unwrap();
        assert_eq!(count_after, 0);
    }
}
