//! System state journal.
//!
//! Stores OS state snapshots for:
//! - Time-travel debugging
//! - Forensic analysis
//! - Configuration rollback

pub use crate::domain::{Change, Snapshot, SnapshotTrigger, SystemState};
use crate::error::{GallifreyError, GallifreyResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;
use tardis_common::SnapshotId;

/// The system state store.
#[derive(Debug)]
pub struct SystemStateStore {
    /// Snapshots indexed by ID.
    snapshots: RwLock<HashMap<SnapshotId, Snapshot>>,
    /// Snapshots by name for quick lookup.
    by_name: RwLock<HashMap<String, SnapshotId>>,
    /// Changes between snapshots.
    changes: RwLock<Vec<Change>>,
}

impl SystemStateStore {
    /// Create a new system state store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
            by_name: RwLock::new(HashMap::new()),
            changes: RwLock::new(Vec::new()),
        }
    }

    /// Take a snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn take_snapshot(
        &self,
        name: &str,
        trigger: SnapshotTrigger,
        state: SystemState,
    ) -> GallifreyResult<SnapshotId> {
        let id = SnapshotId::new();
        let snapshot = Snapshot {
            id,
            name: name.to_string(),
            timestamp: Utc::now(),
            trigger,
            state,
            checksum: String::new(), // TODO: Compute actual checksum
        };

        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut by_name = self
            .by_name
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        snapshots.insert(id, snapshot);
        by_name.insert(name.to_string(), id);

        Ok(id)
    }

    /// Get a snapshot by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_snapshot(&self, id: SnapshotId) -> GallifreyResult<Option<Snapshot>> {
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(snapshots.get(&id).cloned())
    }

    /// Get a snapshot by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_snapshot_by_name(&self, name: &str) -> GallifreyResult<Option<Snapshot>> {
        let by_name = self
            .by_name
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let id = match by_name.get(name) {
            Some(id) => *id,
            None => return Ok(None),
        };

        drop(by_name);

        self.get_snapshot(id)
    }

    /// Find snapshot closest to a timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn find_snapshot_at(&self, timestamp: DateTime<Utc>) -> GallifreyResult<Option<Snapshot>> {
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(snapshots
            .values()
            .filter(|s| s.timestamp <= timestamp)
            .max_by_key(|s| s.timestamp)
            .cloned())
    }

    /// Record a change.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn record_change(&self, change: Change) -> GallifreyResult<()> {
        let mut changes = self
            .changes
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        changes.push(change);

        Ok(())
    }

    /// Get changes between two timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn get_changes(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> GallifreyResult<Vec<Change>> {
        let changes = self
            .changes
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        Ok(changes
            .iter()
            .filter(|c| c.timestamp >= from && c.timestamp <= to)
            .cloned()
            .collect())
    }

    /// List all snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock is poisoned.
    pub fn list_snapshots(&self) -> GallifreyResult<Vec<Snapshot>> {
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut list: Vec<_> = snapshots.values().cloned().collect();
        list.sort_by_key(|s| s.timestamp);

        Ok(list)
    }
}

impl Default for SystemStateStore {
    fn default() -> Self {
        Self::new()
    }
}
