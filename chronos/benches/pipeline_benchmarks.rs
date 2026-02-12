//! Benchmarks for Chronos RAG pipeline operations.

#![allow(clippy::unwrap_used)]
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tardis_chronos::pipeline::{analyzer, ContextAugmenter};

fn bench_query_analyzer(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_analyzer");

    // QueryAnalyzer::new is removed (it was stateless anyway)

    group.bench_function("analyzer::analyze_simple", |b| {
        b.iter(|| black_box(analyzer::analyze("What is Rust?")));
    });

    group.bench_function("analyzer::analyze_temporal", |b| {
        b.iter(|| black_box(analyzer::analyze("What did we discuss yesterday about async?")));
    });

    group.bench_function("analyzer::analyze_complex", |b| {
        b.iter(|| {
            black_box(analyzer::analyze(
            "Last week before the meeting, what changes were made to the authentication system?"
        ))
        });
    });

    group.finish();
}

fn bench_context_augmenter(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_augmenter");

    group.bench_function("ContextAugmenter::new", |b| {
        b.iter(|| black_box(ContextAugmenter::new()));
    });

    let augmenter = ContextAugmenter::new();
    let analysis = analyzer::analyze("What is Rust?").unwrap();

    group.bench_function("ContextAugmenter::augment_empty", |b| {
        b.iter(|| black_box(augmenter.augment("What is Rust?", &[], &analysis)));
    });

    group.finish();
}

criterion_group!(benches, bench_query_analyzer, bench_context_augmenter,);
criterion_main!(benches);
