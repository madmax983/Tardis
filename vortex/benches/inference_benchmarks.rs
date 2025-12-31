//! Benchmarks for Vortex inference operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tardis_vortex::model::{ModelHandle, ModelRegistry};

fn bench_model_registry(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_registry");

    group.bench_function("ModelRegistry::new", |b| {
        b.iter(|| black_box(ModelRegistry::new()));
    });

    let registry = ModelRegistry::new();
    group.bench_function("ModelRegistry::list_empty", |b| {
        b.iter(|| black_box(registry.list()));
    });

    let handle = ModelHandle::new(1);
    group.bench_function("ModelRegistry::is_valid_missing", |b| {
        b.iter(|| black_box(registry.is_valid(handle)));
    });

    group.finish();
}

fn bench_model_handle(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_handle");
    group.throughput(Throughput::Elements(1));

    group.bench_function("ModelHandle::new", |b| {
        b.iter(|| black_box(ModelHandle::new(42)));
    });

    let handle = ModelHandle::new(12345);
    group.bench_function("ModelHandle::raw", |b| {
        b.iter(|| black_box(handle.raw()));
    });

    group.bench_function("ModelHandle::display", |b| {
        b.iter(|| black_box(format!("{}", handle)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_model_registry,
    bench_model_handle,
);
criterion_main!(benches);
