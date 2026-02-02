//! Benchmarks for Chronos RAG pipeline operations.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tardis_chronos::pipeline::{ContextAugmenter, QueryAnalyzer};

fn bench_query_analyzer(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_analyzer");

    group.bench_function("QueryAnalyzer::new", |b| {
        b.iter(|| black_box(QueryAnalyzer::new()));
    });

    let analyzer = QueryAnalyzer::new();

    group.bench_function("QueryAnalyzer::analyze_simple", |b| {
        b.iter(|| black_box(analyzer.analyze("What is Rust?")));
    });

    group.bench_function("QueryAnalyzer::analyze_temporal", |b| {
        b.iter(|| black_box(analyzer.analyze("What did we discuss yesterday about async?")));
    });

    group.bench_function("QueryAnalyzer::analyze_complex", |b| {
        b.iter(|| {
            black_box(analyzer.analyze(
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
    let analyzer = QueryAnalyzer::new();
    let analysis = analyzer.analyze("What is Rust?").unwrap();

    group.bench_function("ContextAugmenter::augment_empty", |b| {
        b.iter(|| black_box(augmenter.augment("What is Rust?", &[], &analysis)));
    });

    group.finish();
}

criterion_group!(benches, bench_query_analyzer, bench_context_augmenter,);
criterion_main!(benches);
