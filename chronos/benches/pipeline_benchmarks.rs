//! Benchmarks for Chronos RAG pipeline operations.

#![allow(clippy::unwrap_used)]
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tardis_chronos::pipeline::{analyze, augment, RagConfig};

fn bench_query_analyzer(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_analyzer");

    group.bench_function("analyze_simple", |b| {
        b.iter(|| black_box(analyze("What is Rust?")));
    });

    group.bench_function("analyze_temporal", |b| {
        b.iter(|| black_box(analyze("What did we discuss yesterday about async?")));
    });

    group.bench_function("analyze_complex", |b| {
        b.iter(|| {
            black_box(analyze(
            "Last week before the meeting, what changes were made to the authentication system?"
        ))
        });
    });

    group.finish();
}

fn bench_context_augmenter(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_augmenter");

    let analysis = analyze("What is Rust?").unwrap();
    let config = RagConfig::default();

    group.bench_function("augment_empty", |b| {
        b.iter(|| black_box(augment("What is Rust?", &[], &analysis, &config)));
    });

    group.finish();
}

criterion_group!(benches, bench_query_analyzer, bench_context_augmenter,);
criterion_main!(benches);
