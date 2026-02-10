//! System state journal.
//!
//! Stores OS state snapshots for:
//! - Time-travel debugging
//! - Forensic analysis
//! - Configuration rollback

use crate::error::{GallifreyError, GallifreyResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
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

    /// Inject a snapshot for testing purposes.
    #[cfg(test)]
    pub fn inject_snapshot(&self, snapshot: Snapshot) {
        let mut snapshots = self.snapshots.write().unwrap();
        let mut by_name = self.by_name.write().unwrap();

        by_name.insert(snapshot.name.clone(), snapshot.id);
        snapshots.insert(snapshot.id, snapshot);
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
    use tardis_common::domain::{ChangeType, ProcessState};
    use tardis_common::id::SnapshotId;

    fn create_dummy_state() -> SystemState {
        SystemState {
            processes: HashMap::from([(
                1,
                ProcessState {
                    pid: 1,
                    name: "init".to_string(),
                    status: "running".to_string(),
                    memory_bytes: 1024,
                    cpu_percent: 0.1,
                },
            )]),
            config: HashMap::new(),
            files: HashMap::new(),
        }
    }

    #[test]
    fn should_create_and_retrieve_snapshot() {
        let store = SystemStateStore::new();
        let state = create_dummy_state();
        let name = "test-snapshot";

        let id = store
            .take_snapshot(name, SnapshotTrigger::Manual, state.clone())
            .expect("failed to take snapshot");

        let retrieved = store.get_snapshot(id).expect("failed to get snapshot");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, name);
        assert!(matches!(retrieved.trigger, SnapshotTrigger::Manual));

        let retrieved_by_name = store
            .get_snapshot_by_name(name)
            .expect("failed to get by name");
        assert!(retrieved_by_name.is_some());
        assert_eq!(retrieved_by_name.unwrap().id, id);
    }

    #[test]
    fn should_find_snapshot_at_timestamp() {
        let store = SystemStateStore::new();
        let base_time = Utc::now();

        // Helper to create snapshot at offset
        let make_snap = |offset_secs: i64, name: &str| {
            let id = SnapshotId::new();
            Snapshot {
                id,
                name: name.to_string(),
                timestamp: base_time + chrono::Duration::seconds(offset_secs),
                trigger: SnapshotTrigger::Manual,
                state: create_dummy_state(),
                checksum: "sum".to_string(),
            }
        };

        let s1 = make_snap(0, "s1"); // T
        let s2 = make_snap(10, "s2"); // T+10
        let s3 = make_snap(20, "s3"); // T+20

        store.inject_snapshot(s1.clone());
        store.inject_snapshot(s2.clone());
        store.inject_snapshot(s3.clone());

        // Exact match
        let found = store.find_snapshot_at(s2.timestamp).unwrap().unwrap();
        assert_eq!(found.name, "s2");

        // Between s1 and s2 (T+5) -> should find s1
        let found = store
            .find_snapshot_at(base_time + chrono::Duration::seconds(5))
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "s1");

        // After s3 (T+100) -> should find s3
        let found = store
            .find_snapshot_at(base_time + chrono::Duration::seconds(100))
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "s3");

        // Before s1 (T-10) -> should find None
        let found = store
            .find_snapshot_at(base_time - chrono::Duration::seconds(10))
            .unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn should_record_and_filter_changes() {
        let store = SystemStateStore::new();
        let base_time = Utc::now();

        let c1 = Change {
            timestamp: base_time,
            path: "/etc/config".to_string(),
            change_type: ChangeType::Create,
            old_value: None,
            new_value: None,
        };

        let c2 = Change {
            timestamp: base_time + chrono::Duration::seconds(10),
            path: "/var/log".to_string(),
            change_type: ChangeType::Update,
            old_value: None,
            new_value: None,
        };

        store.record_change(c1.clone()).unwrap();
        store.record_change(c2.clone()).unwrap();

        // Get all
        let all = store
            .get_changes(
                base_time - chrono::Duration::seconds(1),
                base_time + chrono::Duration::seconds(20),
            )
            .unwrap();
        assert_eq!(all.len(), 2);

        // Get only c1
        let range1 = store
            .get_changes(
                base_time - chrono::Duration::seconds(1),
                base_time + chrono::Duration::seconds(5),
            )
            .unwrap();
        assert_eq!(range1.len(), 1);
        assert_eq!(range1[0].path, "/etc/config");
    }

    #[test]
    fn should_list_snapshots_chronologically() {
        let store = SystemStateStore::new();
        let base_time = Utc::now();

        let s1 = Snapshot {
            id: SnapshotId::new(),
            name: "early".to_string(),
            timestamp: base_time,
            trigger: SnapshotTrigger::Manual,
            state: create_dummy_state(),
            checksum: "".to_string(),
        };

        let s2 = Snapshot {
            id: SnapshotId::new(),
            name: "late".to_string(),
            timestamp: base_time + chrono::Duration::seconds(10),
            trigger: SnapshotTrigger::Manual,
            state: create_dummy_state(),
            checksum: "".to_string(),
        };

        // Insert in reverse order
        store.inject_snapshot(s2.clone());
        store.inject_snapshot(s1.clone());

        let list = store.list_snapshots().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "early");
        assert_eq!(list[1].name, "late");
    }

    #[test]
    fn should_return_none_for_unknown_snapshot() {
        let store = SystemStateStore::new();
        let res = store.get_snapshot(SnapshotId::new()).unwrap();
        assert!(res.is_none());

        let res = store.get_snapshot_by_name("non-existent").unwrap();
        assert!(res.is_none());
    }
}
