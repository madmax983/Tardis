//! System state journal.
//!
//! Stores OS state snapshots for:
//! - Time-travel debugging
//! - Forensic analysis
//! - Configuration rollback

pub use crate::domain::{Change, Snapshot, SnapshotTrigger, SystemState};
use crate::error::{GallifreyError, GallifreyResult};
use chrono::{DateTime, Utc};
use std::collections::{BTreeSet, HashMap};
use std::sync::RwLock;
use tardis_common::SnapshotId;

/// The system state store.
#[derive(Debug)]
pub struct SystemStateStore {
    /// Snapshots indexed by ID.
    snapshots: RwLock<HashMap<SnapshotId, Snapshot>>,
    /// Snapshots by name for quick lookup.
    by_name: RwLock<HashMap<String, SnapshotId>>,
    /// Timeline index for efficient temporal lookups (time -> id).
    /// Uses tuple key (time, id) to handle potential timestamp collisions.
    timeline: RwLock<BTreeSet<(DateTime<Utc>, SnapshotId)>>,
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
            timeline: RwLock::new(BTreeSet::new()),
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

        let mut timeline = self
            .timeline
            .write()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        // Update timeline index
        timeline.insert((snapshot.timestamp, id));

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
        let timeline = self
            .timeline
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        // Bolt optimization: Use BTreeSet index for O(log N) lookup instead of O(N) scan.
        // Look for the last entry <= (timestamp, MAX_UUID)
        let id = match timeline
            .range(..=(timestamp, SnapshotId::max()))
            .next_back()
        {
            Some((_, id)) => *id,
            None => return Ok(None),
        };

        // Important: Drop the timeline lock before acquiring snapshot lock
        drop(timeline);

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
        // Bolt: Acquire locks in consistent order (snapshots -> timeline) to avoid deadlocks
        // with take_snapshot which acquires snapshots (write) -> timeline (write).
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        let timeline = self
            .timeline
            .read()
            .map_err(|_| GallifreyError::StorageError("lock poisoned".to_string()))?;

        // Bolt optimization: Iterate timeline index for O(N) sorted retrieval
        // instead of O(N log N) sort.
        let list: Vec<_> = timeline
            .iter()
            .filter_map(|(_, id)| snapshots.get(id).cloned())
            .collect();

        Ok(list)
    }
}

impl Default for SystemStateStore {
    fn default() -> Self {
        Self::new()
    }
}
