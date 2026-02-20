//! Knowledge graph store.
//!
//! Stores entities, relationships, and facts with:
//! - Embedding vectors for semantic search
//! - Bi-temporal versioning
//! - Source provenance tracking

pub use crate::domain::{Entity, Relationship};
use crate::error::{GallifreyError, GallifreyResult};
use crate::temporal::BiTemporalInterval;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;
use tardis_common::EntityId;

/// The knowledge graph store.
///
/// This store manages the lifecycle of entities and relationships, ensuring
/// thread safety via [`RwLock`]s and history tracking via bi-temporal intervals.
///
/// # Concurrency
///
/// - **Reads**: Concurrent readers are allowed.
/// - **Writes**: Exclusive write access is required for inserts and updates.
///
/// # Performance Warning
///
/// ⚠️ **Missing Index**: This store currently maps `EntityId` -> `Entity`.
/// There is **no secondary index** for entity names or properties.
///
/// To find an entity by name, you must perform a full linear scan of the store
/// (O(N) complexity). For production use cases involving frequent name lookups,
/// consider maintaining an external index or using `EntityId` references.
#[derive(Debug)]
pub struct KnowledgeStore {
    /// Entities indexed by ID.
    entities: RwLock<HashMap<EntityId, Vec<Entity>>>,
    /// Relationships indexed by ID.
    relationships: RwLock<HashMap<EntityId, Vec<Relationship>>>,
}

impl KnowledgeStore {
    /// Create a new knowledge store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            relationships: RwLock::new(HashMap::new()),
        }
    }

    /// Insert an entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use tardis_gallifrey::stores::{KnowledgeStore, Entity};
    /// use tardis_common::id::EntityId;
    /// use tardis_common::temporal::BiTemporalInterval;
    /// use std::collections::HashMap;
    ///
    /// let store = KnowledgeStore::new();
    /// let entity = Entity {
    ///     id: EntityId::new(),
    ///     entity_type: "Test".to_string(),
    ///     name: "Test Entity".to_string(),
    ///     properties: HashMap::new(),
    ///     embedding: None,
    ///     temporal: BiTemporalInterval::now(),
    ///     source: None,
    /// };
    ///
    /// assert!(store.insert_entity(entity).is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn insert_entity(&self, entity: Entity) -> GallifreyResult<EntityId> {
        let id = entity.id;

        let mut entities = self
            .entities
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        if let Some(versions) = entities.get(&id) {
            let now = Utc::now();
            if versions
                .iter()
                .any(|e| e.temporal.transaction_time.is_current_relative_to(now))
            {
                return Err(GallifreyError::EntityAlreadyExists(id.to_string()));
            }
        }

        entities.entry(id).or_default().push(entity);

        Ok(id)
    }

    /// Get the current version of an entity.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_entity(&self, id: EntityId) -> GallifreyResult<Option<Entity>> {
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let now = Utc::now();
        Ok(entities
            .get(&id)
            .and_then(|versions| versions.iter().find(|e| e.temporal.active_at(now, now)))
            .cloned())
    }

    /// Get entity at a specific point in time.
    ///
    /// # Examples
    ///
    /// ```
    /// use tardis_gallifrey::stores::{KnowledgeStore, Entity};
    /// use tardis_common::id::EntityId;
    /// use tardis_common::temporal::BiTemporalInterval;
    /// use chrono::{Utc, Duration};
    /// use std::collections::HashMap;
    ///
    /// let store = KnowledgeStore::new();
    /// let id = EntityId::new();
    /// # let entity = Entity {
    /// #    id,
    /// #    entity_type: "Test".to_string(),
    /// #    name: "Test Entity".to_string(),
    /// #    properties: HashMap::new(),
    /// #    embedding: None,
    /// #    temporal: BiTemporalInterval::now(),
    /// #    source: None,
    /// # };
    /// # store.insert_entity(entity).unwrap();
    ///
    /// let now = Utc::now();
    /// // Query as of now (Valid Time) and as we know it now (Transaction Time)
    /// let current = store.get_entity_at(id, now, now).unwrap();
    /// assert!(current.is_some());
    ///
    /// // Query as of yesterday (Valid Time)
    /// let yesterday = now - Duration::days(1);
    /// let past = store.get_entity_at(id, yesterday, now).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_entity_at(
        &self,
        id: EntityId,
        valid_time: DateTime<Utc>,
        transaction_time: DateTime<Utc>,
    ) -> GallifreyResult<Option<Entity>> {
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(entities.get(&id).and_then(|versions| {
            versions
                .iter()
                .find(|e| e.temporal.active_at(valid_time, transaction_time))
                .cloned()
        }))
    }

    /// Get all versions of an entity (history).
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_entity_history(&self, id: EntityId) -> GallifreyResult<Vec<Entity>> {
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(entities.get(&id).cloned().unwrap_or_default())
    }

    /// Update an entity (creates new version).
    ///
    /// This operation is **non-destructive** (bi-temporal). It implements the "Update" pattern:
    /// 1. **Supersede Old Version**: The current version is marked as "historical" by closing its
    ///    **Transaction Time** at `now`. Its **Valid Time** remains unchanged (it *was* true).
    /// 2. **Create New Version**: A copy is created with the updated properties.
    ///    - **Transaction Time**: Starts `now` (we believe this new version as of now).
    ///    - **Valid Time**: Starts `now` (this new state is effective from now on).
    ///
    /// The result is a chain of versions where:
    /// - `Version 1`: Valid [T0, ∞), Known [T0, T1)
    /// - `Version 2`: Valid [T1, ∞), Known [T1, ∞)
    ///
    /// Use this for state changes (e.g., status changed from "Active" to "Inactive").
    /// For correcting mistakes in the past (e.g., "it was actually Inactive since yesterday"),
    /// you would need a "Correction" operation (not yet exposed via this helper).
    ///
    /// # Examples
    ///
    /// ```
    /// use tardis_gallifrey::stores::{KnowledgeStore, Entity};
    /// use tardis_common::id::EntityId;
    /// use tardis_common::temporal::BiTemporalInterval;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let store = KnowledgeStore::new();
    /// let id = EntityId::new();
    /// # let entity = Entity {
    /// #     id,
    /// #     entity_type: "Person".to_string(),
    /// #     name: "Rose".to_string(),
    /// #     properties: HashMap::new(),
    /// #     embedding: None,
    /// #     temporal: BiTemporalInterval::now(),
    /// #     source: None,
    /// # };
    /// # store.insert_entity(entity).unwrap();
    ///
    /// // 1. Original state: "Rose" (Version 1)
    ///
    /// // 2. Update status to "Bad Wolf"
    /// let mut updates = HashMap::new();
    /// updates.insert("status".to_string(), json!("Bad Wolf"));
    /// store.update_entity(id, updates).unwrap();
    ///
    /// // 3. Verify history
    /// let history = store.get_entity_history(id).unwrap();
    /// assert_eq!(history.len(), 2);
    /// assert!(!history[0].temporal.transaction_time.is_current()); // Old version superseded
    /// assert!(history[1].temporal.transaction_time.is_current());  // New version active
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the entity is not found or the lock is poisoned.
    pub fn update_entity(
        &self,
        id: EntityId,
        updates: HashMap<String, serde_json::Value>,
    ) -> GallifreyResult<()> {
        let mut entities = self
            .entities
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let versions = entities
            .get_mut(&id)
            .ok_or_else(|| GallifreyError::EntityNotFound(id.to_string()))?;

        let now = Utc::now();

        // Find current version and supersede it
        let current = versions
            .iter_mut()
            .find(|e| e.temporal.is_current_relative_to(now))
            .ok_or_else(|| GallifreyError::EntityNotFound(id.to_string()))?;

        // Create new version with updates
        let mut new_version = current.clone();
        for (key, value) in updates {
            new_version.properties.insert(key, value);
        }
        new_version.temporal = BiTemporalInterval::now();

        // Supersede old version
        current.temporal = current.temporal.supersede();

        // Add new version
        versions.push(new_version);

        Ok(())
    }

    /// Insert a relationship.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn insert_relationship(&self, relationship: Relationship) -> GallifreyResult<EntityId> {
        let id = relationship.id;

        let mut relationships = self
            .relationships
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        relationships.entry(id).or_default().push(relationship);

        Ok(id)
    }

    /// Find entities by type.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn find_by_type(&self, entity_type: &str) -> GallifreyResult<Vec<Entity>> {
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let now = Utc::now();

        Ok(entities
            .values()
            .flat_map(|versions| versions.iter())
            .filter(|e| e.temporal.active_at(now, now) && e.entity_type == entity_type)
            .cloned()
            .collect())
    }

    /// Find entities by semantic similarity (placeholder for vector search).
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn semantic_search(
        &self,
        _embedding: &[f32],
        limit: usize,
    ) -> GallifreyResult<Vec<Entity>> {
        // TODO: Implement actual vector similarity search
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let now = Utc::now();

        Ok(entities
            .values()
            .flat_map(|versions| versions.iter())
            .filter(|e| e.temporal.active_at(now, now) && e.embedding.is_some())
            .take(limit)
            .cloned()
            .collect())
    }

    /// Scan all entity histories.
    ///
    /// This method allows iterating over the entire knowledge graph's history
    /// without cloning the underlying storage structure.
    ///
    /// # Performance
    ///
    /// This operation is **O(N)** where N is the number of entities in the store.
    /// It acquires a read lock on the entire store.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    #[cfg(feature = "nova")]
    pub fn scan_history<F>(&self, mut visitor: F) -> GallifreyResult<()>
    where
        F: FnMut(&[Entity]),
    {
        let entities = self
            .entities
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        for history in entities.values() {
            visitor(history);
        }

        Ok(())
    }

    /// Scan all relationships.
    ///
    /// This method allows iterating over all relationships in the knowledge graph.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    #[cfg(feature = "nova")]
    pub fn scan_relationships<F>(&self, mut visitor: F) -> GallifreyResult<()>
    where
        F: FnMut(&[Relationship]),
    {
        let relationships = self
            .relationships
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        for rels in relationships.values() {
            visitor(rels);
        }

        Ok(())
    }
}

impl Default for KnowledgeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;
    use std::thread;
    use std::time::Duration as StdDuration;
    use tardis_common::id::EntityId;
    use tardis_common::temporal::BiTemporalInterval;

    fn create_test_entity(name: &str, entity_type: &str) -> Entity {
        Entity {
            id: EntityId::new(),
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::now(),
            source: Some("test".to_string()),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let store = KnowledgeStore::new();
        let entity = create_test_entity("Test Entity", "Test");
        let id = entity.id;

        store.insert_entity(entity).unwrap();

        let retrieved = store.get_entity(id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Entity");
    }

    #[test]
    fn test_update_entity_bi_temporal() {
        let store = KnowledgeStore::new();
        let entity = create_test_entity("Original", "Test");
        let id = entity.id;

        store.insert_entity(entity).unwrap();

        // Sleep to ensure measurable time difference for transaction time
        thread::sleep(StdDuration::from_millis(10));

        let mut updates = HashMap::new();
        updates.insert("status".to_string(), json!("Updated"));
        store.update_entity(id, updates).unwrap();

        let history = store.get_entity_history(id).unwrap();
        assert_eq!(history.len(), 2);

        // Version 1 (Old)
        let v1 = &history[0];
        assert!(
            !v1.temporal.transaction_time.is_current(),
            "Old version should be superseded"
        );
        assert!(
            v1.temporal.valid_time.is_current(),
            "Old version valid time should remain current (unless corrected)"
        );

        // Version 2 (New)
        let v2 = &history[1];
        assert!(
            v2.temporal.transaction_time.is_current(),
            "New version should be current"
        );
        assert_eq!(v2.properties.get("status"), Some(&json!("Updated")));
        assert_eq!(v2.name, "Original", "Name should be preserved");
    }

    #[test]
    fn test_get_entity_at() {
        let store = KnowledgeStore::new();

        // T0: Before insert
        let t0 = Utc::now();
        thread::sleep(StdDuration::from_millis(10));

        let entity = create_test_entity("Time Traveler", "Person");
        let id = entity.id;
        store.insert_entity(entity).unwrap();

        // T1: After insert, before update
        thread::sleep(StdDuration::from_millis(10));
        let t1 = Utc::now();
        thread::sleep(StdDuration::from_millis(10));

        let mut updates = HashMap::new();
        updates.insert("status".to_string(), json!("Changed"));
        store.update_entity(id, updates).unwrap();

        // T2: After update
        thread::sleep(StdDuration::from_millis(10));
        let t2 = Utc::now();

        // Query at T0
        let at_t0 = store.get_entity_at(id, t0, t0).unwrap();
        assert!(at_t0.is_none());

        // Query at T1
        let at_t1 = store.get_entity_at(id, t1, t1).unwrap();
        assert!(at_t1.is_some());
        assert!(at_t1.unwrap().properties.is_empty());

        // Query at T2
        let at_t2 = store.get_entity_at(id, t2, t2).unwrap();
        assert!(at_t2.is_some());
        assert_eq!(
            at_t2.unwrap().properties.get("status"),
            Some(&json!("Changed"))
        );
    }

    #[test]
    fn test_find_by_type() {
        let store = KnowledgeStore::new();
        store
            .insert_entity(create_test_entity("A", "Type1"))
            .unwrap();
        store
            .insert_entity(create_test_entity("B", "Type2"))
            .unwrap();
        store
            .insert_entity(create_test_entity("C", "Type1"))
            .unwrap();

        let type1 = store.find_by_type("Type1").unwrap();
        assert_eq!(type1.len(), 2);

        let type2 = store.find_by_type("Type2").unwrap();
        assert_eq!(type2.len(), 1);
        assert_eq!(type2[0].name, "B");
    }

    #[test]
    fn test_semantic_search_stub() {
        let store = KnowledgeStore::new();
        let mut entity = create_test_entity("Embedded", "Test");
        entity.embedding = Some(vec![0.1, 0.2, 0.3]);
        store.insert_entity(entity).unwrap();

        let mut no_embed = create_test_entity("No Embed", "Test");
        no_embed.embedding = None;
        store.insert_entity(no_embed).unwrap();

        let results = store.semantic_search(&[0.1, 0.2, 0.3], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Embedded");
    }

    #[test]
    fn test_update_non_existent_entity() {
        let store = KnowledgeStore::new();
        let id = EntityId::new();
        let updates = HashMap::new();

        let result = store.update_entity(id, updates);
        assert!(result.is_err());
        assert!(matches!(result, Err(GallifreyError::EntityNotFound(_))));
    }

    #[test]
    fn test_get_non_existent_entity() {
        let store = KnowledgeStore::new();
        let id = EntityId::new();
        assert!(store.get_entity(id).unwrap().is_none());
    }
}
