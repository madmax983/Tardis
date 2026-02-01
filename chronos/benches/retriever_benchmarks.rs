use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;
use tardis_chronos::pipeline::{AnalyzedQuery, QueryIntent, RagConfig, Retriever};
use tardis_common::EntityId;
use tardis_gallifrey::Gallifrey;

fn bench_retriever(c: &mut Criterion) {
    let mut group = c.benchmark_group("retriever");

    // Setup (sync, outside benchmark loop)
    let gallifrey = Arc::new(Gallifrey::new());
    let retriever = Retriever::new(gallifrey.clone());

    // Populate data
    // Add a session with messages
    let session_id = gallifrey.conversation().create_session().unwrap();
    for i in 0..10 {
        gallifrey
            .conversation()
            .add_message(tardis_gallifrey::stores::Message {
                id: EntityId::new(),
                session_id,
                role: tardis_gallifrey::stores::Role::User,
                content: format!("Message {}", i),
                timestamp: Utc::now(),
                embedding: Some(vec![0.1; 128]),
                entity_refs: vec![],
            })
            .unwrap();
    }

    // Add entities
    for i in 0..10 {
        gallifrey
            .knowledge()
            .insert_entity(tardis_gallifrey::stores::Entity {
                id: EntityId::new(),
                entity_type: "Fact".to_string(),
                name: format!("Fact {}", i),
                properties: std::collections::HashMap::new(),
                embedding: Some(vec![0.1; 128]),
                temporal: tardis_gallifrey::BiTemporalInterval::now(),
                source: None,
            })
            .unwrap();
    }

    let query = AnalyzedQuery {
        text: "test".to_string(),
        intent: QueryIntent::Chat,
        temporal_refs: vec![],
        temporal_description: None,
        entities: vec![],
    };

    let config = RagConfig {
        session_id: Some(session_id),
        max_context_items: 20,
        ..Default::default()
    };

    group.bench_function("Retriever::retrieve", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                black_box(retriever.retrieve(&query, &config).await.unwrap());
            })
    });

    group.finish();
}

criterion_group!(benches, bench_retriever);
criterion_main!(benches);
