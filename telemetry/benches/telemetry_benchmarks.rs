//! Telemetry benchmarks.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tardis_telemetry::types::{EventType, Level, SpanId, Subsystem, TelemetryEntry, TraceId};

fn bench_trace_id_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("TraceId");

    group.bench_function("generate", |b| {
        b.iter(|| black_box(TraceId::generate()));
    });

    group.bench_function("display", |b| {
        let id = TraceId::generate();
        b.iter(|| black_box(format!("{id}")));
    });

    group.finish();
}

fn bench_span_id_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("SpanId");

    group.bench_function("generate", |b| {
        b.iter(|| black_box(SpanId::generate()));
    });

    group.finish();
}

fn bench_telemetry_entry(c: &mut Criterion) {
    let mut group = c.benchmark_group("TelemetryEntry");

    group.bench_function("create_log", |b| {
        b.iter(|| black_box(TelemetryEntry::log(Level::Info, Subsystem::Vortex)));
    });

    group.bench_function("create_full", |b| {
        b.iter(|| {
            black_box(TelemetryEntry::new(
                Level::Info,
                Subsystem::Vortex,
                EventType::InferenceStart,
                TraceId::generate(),
                SpanId::generate(),
                SpanId::NONE,
            ))
        });
    });

    group.finish();
}

fn bench_subsystem_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Subsystem");

    group.bench_function("from_target_vortex", |b| {
        b.iter(|| black_box(Subsystem::from_target("tardis_vortex::inference")));
    });

    group.bench_function("from_target_unknown", |b| {
        b.iter(|| black_box(Subsystem::from_target("random_crate::module")));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_trace_id_generation,
    bench_span_id_generation,
    bench_telemetry_entry,
    bench_subsystem_parsing,
);
criterion_main!(benches);
