#![allow(missing_docs, clippy::unwrap_used)]
use chrono::Duration;
use std::collections::HashMap;
use tardis_gallifrey::domain::{SnapshotTrigger, SystemState};
use tardis_gallifrey::stores::SystemStateStore;

#[test]
fn test_find_snapshot_at_timeline() {
    let store = SystemStateStore::new();
    let state = SystemState {
        processes: HashMap::new(),
        config: HashMap::new(),
        files: HashMap::new(),
    };

    // s1
    let id1 = store
        .take_snapshot("s1", SnapshotTrigger::Manual, state.clone())
        .unwrap();

    // Ensure distinct timestamp
    std::thread::sleep(std::time::Duration::from_millis(10));

    // s2
    let id2 = store
        .take_snapshot("s2", SnapshotTrigger::Manual, state.clone())
        .unwrap();

    // Ensure distinct timestamp
    std::thread::sleep(std::time::Duration::from_millis(10));

    // s3
    let id3 = store
        .take_snapshot("s3", SnapshotTrigger::Manual, state.clone())
        .unwrap();

    let s1 = store.get_snapshot(id1).unwrap().unwrap();
    let s2 = store.get_snapshot(id2).unwrap().unwrap();
    let s3 = store.get_snapshot(id3).unwrap().unwrap();

    // Check exact match
    let f1 = store.find_snapshot_at(s1.timestamp).unwrap().unwrap();
    assert_eq!(f1.id, id1);

    // Check slightly after s1, before s2
    let mid_time = s1.timestamp + (s2.timestamp - s1.timestamp) / 2;
    let f1_later = store.find_snapshot_at(mid_time).unwrap().unwrap();
    assert_eq!(f1_later.id, id1);

    // Check exact match s2
    let f2 = store.find_snapshot_at(s2.timestamp).unwrap().unwrap();
    assert_eq!(f2.id, id2);

    // Check after s3
    let f3 = store
        .find_snapshot_at(s3.timestamp + Duration::hours(1))
        .unwrap()
        .unwrap();
    assert_eq!(f3.id, id3);

    // Check before s1 (should be None)
    let f_none = store
        .find_snapshot_at(s1.timestamp - Duration::seconds(1))
        .unwrap();
    assert!(f_none.is_none());
}

#[test]
fn test_list_snapshots_order() {
    let store = SystemStateStore::new();
    let state = SystemState {
        processes: HashMap::new(),
        config: HashMap::new(),
        files: HashMap::new(),
    };

    let id1 = store
        .take_snapshot("s1", SnapshotTrigger::Manual, state.clone())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let id2 = store
        .take_snapshot("s2", SnapshotTrigger::Manual, state.clone())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let id3 = store
        .take_snapshot("s3", SnapshotTrigger::Manual, state.clone())
        .unwrap();

    let list = store.list_snapshots().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, id1);
    assert_eq!(list[1].id, id2);
    assert_eq!(list[2].id, id3);

    // Verify timestamps are strictly increasing
    assert!(list[0].timestamp < list[1].timestamp);
    assert!(list[1].timestamp < list[2].timestamp);
}
