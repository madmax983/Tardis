//! Benchmarks for shell router operations.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use tardis_shell::router;

fn bench_router_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_routing");
    group.throughput(Throughput::Elements(1));

    // router::route is now a free function, so no instance needed

    // Shell commands
    group.bench_function("route_shell_command", |b| {
        b.iter(|| black_box(router::route("!ls -la")));
    });

    // Built-in commands
    group.bench_function("route_builtin_help", |b| {
        b.iter(|| black_box(router::route("help")));
    });

    group.bench_function("route_builtin_history", |b| {
        b.iter(|| black_box(router::route("history")));
    });

    group.bench_function("route_builtin_with_args", |b| {
        b.iter(|| black_box(router::route("remember this is important")));
    });

    // Time travel
    group.bench_function("route_time_travel", |b| {
        b.iter(|| black_box(router::route("@yesterday what did we discuss")));
    });

    // Direct query
    group.bench_function("route_direct_query", |b| {
        b.iter(|| black_box(router::route("?explain async await")));
    });

    // Chronos query (default)
    group.bench_function("route_chronos_simple", |b| {
        b.iter(|| black_box(router::route("What is Rust?")));
    });

    group.bench_function("route_chronos_temporal", |b| {
        b.iter(|| black_box(router::route("What did we discuss yesterday about the project?")));
    });

    group.bench_function("route_chronos_long", |b| {
        b.iter(|| black_box(router::route(
            "Can you explain how the authentication system works and what changes were made last week?"
        )));
    });

    group.finish();
}

criterion_group!(benches, bench_router_routing,);
criterion_main!(benches);
