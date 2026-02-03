//! Benchmarks for ID generation and temporal operations.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use tardis_common::temporal::{BiTemporalInterval, TimeRange};
use tardis_common::{EntityId, ModelHandle, SessionId, SnapshotId};

fn bench_entity_id_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("id_creation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("EntityId::new", |b| {
        b.iter(|| black_box(EntityId::new()));
    });

    group.bench_function("SessionId::new", |b| {
        b.iter(|| black_box(SessionId::new()));
    });

    group.bench_function("SnapshotId::new", |b| {
        b.iter(|| black_box(SnapshotId::new()));
    });

    group.bench_function("ModelHandle::new", |b| {
        b.iter(|| black_box(ModelHandle::new(42)));
    });

    group.finish();
}

fn bench_temporal_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("temporal_operations");

    group.bench_function("BiTemporalInterval::now", |b| {
        b.iter(|| black_box(BiTemporalInterval::now()));
    });

    let interval = BiTemporalInterval::now();
    group.bench_function("BiTemporalInterval::is_current", |b| {
        b.iter(|| black_box(interval.is_current()));
    });

    group.bench_function("BiTemporalInterval::supersede", |b| {
        let interval = BiTemporalInterval::now();
        b.iter(|| black_box(interval.supersede()));
    });

    group.bench_function("TimeRange::from_now", |b| {
        b.iter(|| black_box(TimeRange::from_now()));
    });

    let range = TimeRange::from_now();
    let now = chrono::Utc::now();
    group.bench_function("TimeRange::contains", |b| {
        b.iter(|| black_box(range.contains(now)));
    });

    group.finish();
}

fn bench_id_display(c: &mut Criterion) {
    let mut group = c.benchmark_group("id_display");

    let entity_id = EntityId::new();
    group.bench_function("EntityId::to_string", |b| {
        b.iter(|| black_box(entity_id.to_string()));
    });

    let session_id = SessionId::new();
    group.bench_function("SessionId::to_string", |b| {
        b.iter(|| black_box(session_id.to_string()));
    });

    let handle = ModelHandle::new(12345);
    group.bench_function("ModelHandle::to_string", |b| {
        b.iter(|| black_box(handle.to_string()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_id_creation,
    bench_temporal_operations,
    bench_id_display,
);
criterion_main!(benches);
