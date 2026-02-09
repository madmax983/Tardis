//! Knowledge graph store.
//!
//! Stores entities, relationships, and facts with:
//! - Embedding vectors for semantic search
//! - Bi-temporal versioning
//! - Source provenance tracking

use crate::error::{GallifreyError, GallifreyResult};
use crate::temporal::BiTemporalInterval;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;
pub use tardis_common::domain::{Entity, Relationship};
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

        Ok(entities
            .get(&id)
            .and_then(|versions| versions.iter().find(|e| e.temporal.is_current()))
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

        // Find current version and supersede it
        let current = versions
            .iter_mut()
            .find(|e| e.temporal.is_current())
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

        Ok(entities
            .values()
            .flat_map(|versions| versions.iter())
            .filter(|e| e.temporal.is_current() && e.entity_type == entity_type)
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

        Ok(entities
            .values()
            .flat_map(|versions| versions.iter())
            .filter(|e| e.temporal.is_current() && e.embedding.is_some())
            .take(limit)
            .cloned()
            .collect())
    }

    /// Scan all entity histories.
    ///
    /// This method allows iterating over the entire knowledge graph's history
    /// without cloning the underlying storage structure.
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
}

impl Default for KnowledgeStore {
    fn default() -> Self {
        Self::new()
    }
}
