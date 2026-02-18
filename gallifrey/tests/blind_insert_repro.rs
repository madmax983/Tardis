#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::HashMap;
use tardis_common::id::EntityId;
use tardis_common::temporal::BiTemporalInterval;
use tardis_gallifrey::stores::{Entity, KnowledgeStore};
use tardis_gallifrey::GallifreyError;

#[test]
fn test_blind_insert_fails_if_exists() {
    let store = KnowledgeStore::new();
    let id = EntityId::new();

    let e1 = Entity {
        id,
        entity_type: "Test".to_string(),
        name: "Entity 1".to_string(),
        properties: HashMap::new(),
        embedding: None,
        temporal: BiTemporalInterval::now(),
        source: None,
    };

    let e2 = Entity {
        id,
        entity_type: "Test".to_string(),
        name: "Entity 2".to_string(),
        properties: HashMap::new(),
        embedding: None,
        temporal: BiTemporalInterval::now(),
        source: None,
    };

    // 1. Insert first entity
    store
        .insert_entity(e1)
        .expect("First insert should succeed");

    // 2. Insert second entity - SHOULD FAIL with EntityAlreadyExists
    let result = store.insert_entity(e2);

    match result {
        Err(GallifreyError::EntityAlreadyExists(_)) => {
            // Success! The fix is working.
        }
        Ok(_) => {
            panic!(
                "Blind insert succeeded! This creates data corruption (multiple current versions)."
            );
        }
        Err(e) => {
            panic!("Unexpected error: {e:?}");
        }
    }

    // 3. Verify no corruption (still only 1 entity in history)
    let history = store.get_entity_history(id).unwrap();
    assert_eq!(
        history.len(),
        1,
        "Should only have 1 version if insert failed"
    );
}
