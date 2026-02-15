//! Timeline simulation for "What-If" scenarios.
//!
//! This module provides a `Timeline` struct that acts as a copy-on-write overlay
//! on top of a base `Gallifrey` instance. It allows experimenting with state changes
//! (inserts, updates, deletes) without affecting the main timeline.

use crate::domain::Entity;
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
    deleted: RwLock<HashSet<EntityId>>,
}

impl Timeline {
    /// Create a new timeline branching off the given base.
    #[must_use]
    pub fn new(base: Arc<Gallifrey>) -> Self {
        Self {
            base,
            overlay: Arc::new(KnowledgeStore::new()),
            deleted: RwLock::new(HashSet::new()),
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
            .deleted
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
        self.deleted
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
            .deleted
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
        self.deleted
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?
            .insert(id);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::Entity;
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
}
