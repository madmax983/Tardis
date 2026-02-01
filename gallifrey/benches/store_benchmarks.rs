//! Benchmarks for Gallifrey store operations.

#![allow(clippy::expect_used)]
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;
use tardis_common::EntityId;
use tardis_gallifrey::BiTemporalInterval;
use tardis_gallifrey::stores::{ConversationStore, Entity, KnowledgeStore};

fn bench_knowledge_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("knowledge_store");

    group.bench_function("KnowledgeStore::new", |b| {
        b.iter(|| black_box(KnowledgeStore::new()));
    });

    let store = KnowledgeStore::new();
    group.bench_function("KnowledgeStore::find_by_type_empty", |b| {
        b.iter(|| black_box(store.find_by_type("Concept")));
    });

    // Benchmark insert
    group.throughput(Throughput::Elements(1));
    group.bench_function("KnowledgeStore::insert_entity", |b| {
        let store = KnowledgeStore::new();
        b.iter(|| {
            let entity = Entity {
                id: EntityId::new(),
                entity_type: "Concept".to_string(),
                name: "Test".to_string(),
                properties: HashMap::new(),
                embedding: None,
                temporal: BiTemporalInterval::now(),
                source: None,
            };
            black_box(store.insert_entity(entity))
        });
    });

    // Benchmark get after insert
    let store = KnowledgeStore::new();
    let entity = Entity {
        id: EntityId::new(),
        entity_type: "Concept".to_string(),
        name: "Test".to_string(),
        properties: HashMap::new(),
        embedding: None,
        temporal: BiTemporalInterval::now(),
        source: None,
    };
    let id = store.insert_entity(entity).expect("failed to insert entity");
    group.bench_function("KnowledgeStore::get_entity", |b| {
        b.iter(|| black_box(store.get_entity(id)));
    });

    group.finish();
}

fn bench_conversation_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("conversation_store");

    group.bench_function("ConversationStore::new", |b| {
        b.iter(|| black_box(ConversationStore::new()));
    });

    let store = ConversationStore::new();
    group.bench_function("ConversationStore::create_session", |b| {
        b.iter(|| black_box(store.create_session()));
    });

    let store = ConversationStore::new();
    let session_id = store.create_session().expect("failed to create session");
    group.bench_function("ConversationStore::get_session", |b| {
        b.iter(|| black_box(store.get_session(session_id)));
    });

    group.bench_function("ConversationStore::list_sessions", |b| {
        b.iter(|| black_box(store.list_sessions()));
    });

    group.finish();
}

fn bench_bi_temporal(c: &mut Criterion) {
    let mut group = c.benchmark_group("bi_temporal");

    group.bench_function("BiTemporalInterval::now", |b| {
        b.iter(|| black_box(BiTemporalInterval::now()));
    });

    let interval = BiTemporalInterval::now();
    group.bench_function("BiTemporalInterval::is_current", |b| {
        b.iter(|| black_box(interval.is_current()));
    });

    let now = chrono::Utc::now();
    group.bench_function("BiTemporalInterval::active_at", |b| {
        b.iter(|| black_box(interval.active_at(now, now)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_knowledge_store,
    bench_conversation_store,
    bench_bi_temporal,
);
criterion_main!(benches);
