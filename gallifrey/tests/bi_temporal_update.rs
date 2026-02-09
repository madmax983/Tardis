//! Bi-temporal update regression tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use tardis_gallifrey::stores::{KnowledgeStore, Entity};
use tardis_common::id::EntityId;
use tardis_common::temporal::BiTemporalInterval;
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_bi_temporal_update_logic() {
    let store = KnowledgeStore::new();
    let id = EntityId::new();
    let entity = Entity {
        id,
        entity_type: "Test".to_string(),
        name: "Test Entity".to_string(),
        properties: HashMap::from([
            ("version".to_string(), json!(1))
        ]),
        embedding: None,
        temporal: BiTemporalInterval::now(),
        source: None,
    };

    // 1. Insert original entity
    store.insert_entity(entity.clone()).expect("Insert failed");

    // 2. Wait a tiny bit to ensure timestamps differ (though not strictly necessary with superseding)
    std::thread::sleep(std::time::Duration::from_millis(1));

    // 3. Update entity
    let mut updates = HashMap::new();
    updates.insert("version".to_string(), json!(2));
    store.update_entity(id, updates).expect("Update failed");

    // 4. Inspect history directly (using get_entity_history which returns all versions)
    let history = store.get_entity_history(id).expect("Get history failed");

    // We expect 2 versions:
    // - Version 1: Transaction Time closed (superseded)
    // - Version 2: Transaction Time open (current)
    assert_eq!(history.len(), 2, "Should have 2 versions");

    let v1 = &history[0];
    let v2 = &history[1];

    // Check V1 (Old)
    assert_eq!(v1.properties.get("version").unwrap(), &json!(1));
    assert!(!v1.temporal.transaction_time.is_current(), "V1 transaction time should be closed");
    assert!(v1.temporal.transaction_time.end.is_some(), "V1 should have end time");

    // Check V2 (New)
    assert_eq!(v2.properties.get("version").unwrap(), &json!(2));
    assert!(v2.temporal.transaction_time.is_current(), "V2 transaction time should be open");
    assert!(v2.temporal.transaction_time.end.is_none(), "V2 should not have end time");

    // Check Valid Time
    // V2's valid time should start at 'now' (when update happened)
    // V1's valid time should be unchanged from creation.
    // V2 transaction time starts at 'now'.

    assert!(v2.temporal.valid_time.start > v1.temporal.valid_time.start, "V2 valid time should be strictly greater than V1");
    assert!(v2.temporal.transaction_time.start > v1.temporal.transaction_time.start, "V2 transaction time should be strictly greater than V1");
}
