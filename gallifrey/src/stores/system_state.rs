//! System state journal.
//!
//! Stores OS state snapshots for:
//! - Time-travel debugging
//! - Forensic analysis
//! - Configuration rollback

use crate::error::{GallifreyError, GallifreyResult};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap};
use std::sync::RwLock;
pub use tardis_common::domain::{Change, Snapshot, SnapshotTrigger, SystemState};
use tardis_common::SnapshotId;

/// The system state store.
#[derive(Debug)]
pub struct SystemStateStore {
    /// Snapshots indexed by ID.
    snapshots: RwLock<HashMap<SnapshotId, Snapshot>>,
    /// Snapshots by name for quick lookup.
    by_name: RwLock<HashMap<String, SnapshotId>>,
    /// Snapshots indexed by timestamp for range queries.
    by_timestamp: RwLock<BTreeMap<DateTime<Utc>, SnapshotId>>,
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
            by_timestamp: RwLock::new(BTreeMap::new()),
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

        let timestamp = snapshot.timestamp;

        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut by_name = self
            .by_name
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut by_timestamp = self
            .by_timestamp
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        snapshots.insert(id, snapshot);
        by_name.insert(name.to_string(), id);
        by_timestamp.insert(timestamp, id);

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
        let by_timestamp = self
            .by_timestamp
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        // Find the latest snapshot at or before the timestamp
        let id = match by_timestamp.range(..=timestamp).next_back() {
            Some((_, id)) => *id,
            None => return Ok(None),
        };

        drop(by_timestamp);

        self.get_snapshot(id)
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

    /// Inject a snapshot (for testing only).
    #[cfg(test)]
    pub fn inject_snapshot(&self, snapshot: Snapshot) -> GallifreyResult<()> {
        let id = snapshot.id;
        let timestamp = snapshot.timestamp;
        let name = snapshot.name.clone();

        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut by_name = self
            .by_name
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let mut by_timestamp = self
            .by_timestamp
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        snapshots.insert(id, snapshot);
        by_name.insert(name, id);
        by_timestamp.insert(timestamp, id);

        Ok(())
    }
}

impl Default for SystemStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashMap;
    use tardis_common::domain::SystemState;

    #[test]
    fn test_find_snapshot_at_optimization() {
        let store = SystemStateStore::new();
        let base_time = Utc.timestamp_opt(1600000000, 0).unwrap();

        // Create 3 snapshots
        let s1 = Snapshot {
            id: SnapshotId::new(),
            name: "s1".to_string(),
            timestamp: base_time,
            trigger: SnapshotTrigger::Manual,
            state: SystemState {
                processes: HashMap::new(),
                config: HashMap::new(),
                files: HashMap::new(),
            },
            checksum: String::new(),
        };

        let s2 = Snapshot {
            id: SnapshotId::new(),
            name: "s2".to_string(),
            timestamp: base_time + chrono::Duration::hours(1),
            trigger: SnapshotTrigger::Manual,
            state: SystemState {
                processes: HashMap::new(),
                config: HashMap::new(),
                files: HashMap::new(),
            },
            checksum: String::new(),
        };

        let s3 = Snapshot {
            id: SnapshotId::new(),
            name: "s3".to_string(),
            timestamp: base_time + chrono::Duration::hours(2),
            trigger: SnapshotTrigger::Manual,
            state: SystemState {
                processes: HashMap::new(),
                config: HashMap::new(),
                files: HashMap::new(),
            },
            checksum: String::new(),
        };

        store.inject_snapshot(s1.clone()).unwrap();
        store.inject_snapshot(s2.clone()).unwrap();
        store.inject_snapshot(s3.clone()).unwrap();

        // Query before first snapshot -> None
        assert!(store
            .find_snapshot_at(base_time - chrono::Duration::seconds(1))
            .unwrap()
            .is_none());

        // Query at first snapshot -> s1
        let res = store.find_snapshot_at(base_time).unwrap().unwrap();
        assert_eq!(res.id, s1.id);

        // Query between s1 and s2 -> s1
        let res = store
            .find_snapshot_at(base_time + chrono::Duration::minutes(30))
            .unwrap()
            .unwrap();
        assert_eq!(res.id, s1.id);

        // Query at s2 -> s2
        let res = store
            .find_snapshot_at(base_time + chrono::Duration::hours(1))
            .unwrap()
            .unwrap();
        assert_eq!(res.id, s2.id);

        // Query after s3 -> s3
        let res = store
            .find_snapshot_at(base_time + chrono::Duration::hours(3))
            .unwrap()
            .unwrap();
        assert_eq!(res.id, s3.id);
    }
}
