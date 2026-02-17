#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tardis_common::id::EntityId;
    use tardis_gallifrey::domain::Entity;
    use tardis_gallifrey::{BiTemporalInterval, KnowledgeStore, TimeRange};

    #[test]
    fn get_entity_should_not_return_future_entity() {
        let store = KnowledgeStore::new();
        let now = Utc::now();
        let future_time = now + Duration::hours(1);

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "FutureFact".to_string(),
            name: "Will happen".to_string(),
            properties: HashMap::new(),
            embedding: None,
            // Valid ONLY in the future
            temporal: BiTemporalInterval::with_valid_time(TimeRange::starting_at(future_time)),
            source: None,
        };

        let id = entity.id;
        store.insert_entity(entity).expect("Insert failed");

        // This should return None because the entity is not valid YET.
        let retrieved = store.get_entity(id).expect("Get entity failed");

        assert!(retrieved.is_none(), "get_entity() returned a future entity! It should only return currently valid entities.");
    }

    #[test]
    fn find_by_type_should_not_return_future_entity() {
        let store = KnowledgeStore::new();
        let now = Utc::now();
        let future_time = now + Duration::hours(1);

        let entity = Entity {
            id: EntityId::new(),
            entity_type: "FutureFact".to_string(),
            name: "Will happen".to_string(),
            properties: HashMap::new(),
            embedding: None,
            temporal: BiTemporalInterval::with_valid_time(TimeRange::starting_at(future_time)),
            source: None,
        };

        store.insert_entity(entity).expect("Insert failed");

        let results = store.find_by_type("FutureFact").expect("Find failed");
        assert!(
            results.is_empty(),
            "find_by_type() returned a future entity!"
        );
    }
}
