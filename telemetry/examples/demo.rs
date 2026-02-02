//! Tardis Telemetry Demo
//!
//! Run with: `cargo run --example demo -p tardis-telemetry`
//!
//! This demo shows:
//! - Initializing the telemetry system
//! - Emitting spans and events with tracing
//! - Recording metrics (counters, gauges, histograms)
//! - Querying Gallifrey for temporal data

use std::time::Duration;
use tardis_telemetry::{counter, gauge, histogram, init, TelemetryConfig};
use tracing::{info, info_span, instrument, warn};

/// Simulated inference request
#[derive(Debug)]
struct InferenceRequest {
    prompt: String,
    max_tokens: usize,
}

/// Simulated inference response
#[derive(Debug)]
struct InferenceResponse {
    text: String,
    tokens_generated: usize,
    latency_ms: u64,
}

/// Simulates an LLM inference call with full instrumentation
#[instrument(skip(request), fields(prompt_len = request.prompt.len()))]
async fn run_inference(request: InferenceRequest) -> InferenceResponse {
    info!(max_tokens = request.max_tokens, "Starting inference");

    // Track active inferences
    gauge!("vortex.active_inferences").inc();

    // Simulate tokenization
    let _tokenize_span = info_span!("tokenize").entered();
    tokio::time::sleep(Duration::from_millis(5)).await;
    drop(_tokenize_span);

    // Simulate model forward pass
    let forward_span = info_span!("forward_pass", layers = 32).entered();
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(forward_span);

    // Simulate token generation
    let tokens = 42;
    for i in 0..tokens {
        if i % 10 == 0 {
            info!(token_idx = i, "Generated token batch");
        }
        counter!("vortex.tokens_generated").inc();
        tokio::time::sleep(Duration::from_millis(2)).await;
    }

    let latency_ms = 50 + 42 * 2 + 5; // Simulated total

    // Record metrics
    counter!("vortex.inference_total").inc();
    histogram!("vortex.inference_latency_ms").record(latency_ms);
    gauge!("vortex.active_inferences").dec();

    info!(tokens_generated = tokens, latency_ms, "Inference complete");

    InferenceResponse {
        text: "The answer to life, the universe, and everything is 42.".into(),
        tokens_generated: tokens,
        latency_ms,
    }
}

/// Simulates a RAG query with retrieval and augmentation
#[instrument]
async fn run_rag_query(query: &str) -> String {
    info!("Processing RAG query");

    // Retrieval phase
    let retrieval_span = info_span!("retrieval", sources = 3).entered();
    tokio::time::sleep(Duration::from_millis(15)).await;
    counter!("chronos.retrieval_total").inc();
    histogram!("chronos.retrieval_latency_ms").record(15);
    drop(retrieval_span);

    // Context augmentation
    let augment_span = info_span!("augmentation").entered();
    tokio::time::sleep(Duration::from_millis(5)).await;
    drop(augment_span);

    // Call inference
    let response = run_inference(InferenceRequest {
        prompt: format!("Context: [retrieved docs]\n\nQuery: {query}"),
        max_tokens: 100,
    })
    .await;

    counter!("chronos.rag_query_total").inc();

    response.text
}

/// Simulates a Gallifrey temporal query
#[instrument]
async fn query_knowledge_store(entity: &str) {
    info!(entity, "Querying knowledge store");

    let query_span = info_span!("gallifrey_query", entity_type = "fact").entered();
    tokio::time::sleep(Duration::from_millis(8)).await;

    counter!("gallifrey.query_total").inc();
    histogram!("gallifrey.query_latency_us").record(8000);
    gauge!("gallifrey.entity_count").set(1337);

    drop(query_span);

    info!("Query complete, found 3 temporal versions");
}

/// Demonstrates system metrics
fn record_system_metrics() {
    // Simulated system stats
    gauge!("system.heap_used_bytes").set(1024 * 1024 * 256); // 256 MB
    gauge!("system.process_count").set(42);
    gauge!("system.gpu_memory_bytes").set(1024 * 1024 * 1024 * 4); // 4 GB

    info!(
        heap_mb = 256,
        processes = 42,
        gpu_gb = 4,
        "System metrics recorded"
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry with Gallifrey storage enabled
    let config = TelemetryConfig {
        gallifrey_enabled: true,
        console_enabled: false, // Disable console to reduce noise
        json_output: false,
        env_filter: Some("info".to_string()), // Capture all info+ spans
        ..Default::default()
    };

    let handle = init(config)?;

    // Verify store is connected
    println!("Store connected: {}", handle.store().is_some());

    // Test direct store access
    if let Some(store) = handle.store() {
        println!("Initial span count: {}", store.span_count());
    }

    println!("\n========================================");
    println!("  Tardis Telemetry Demo");
    println!("========================================\n");

    // Create a root span for the entire demo
    let demo_span = info_span!("demo", version = "0.1.0").entered();

    info!("Telemetry system initialized");

    // Record initial system metrics
    record_system_metrics();

    // Simulate some operations
    println!("\n--- Running RAG Query ---\n");
    let result = run_rag_query("What is the meaning of life?").await;
    println!("\nRAG Result: {result}\n");

    // Query knowledge store
    println!("--- Querying Knowledge Store ---\n");
    query_knowledge_store("meaning_of_life").await;

    // Simulate an error condition
    println!("\n--- Simulating Warning ---\n");
    warn!(
        error_code = "CACHE_MISS",
        "KV cache miss, regenerating context"
    );

    // Show metrics summary
    println!("\n--- Metrics Summary ---");
    println!("(Metrics are recorded internally - export to Prometheus/OTLP for visualization)\n");

    drop(demo_span);

    // Allow async storage tasks to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Demonstrate Gallifrey temporal queries
    if let Some(store) = handle.store() {
        println!("--- Gallifrey Temporal Store ---\n");

        // Show total counts
        println!("Total spans in store: {}", store.span_count());
        println!("Total events in store: {}", store.event_count());

        // Query what was happening around now (5 second window to catch everything)
        let query_time = chrono::Utc::now();
        let context = store.context_around(query_time, 5000);

        println!("Spans recorded: {}", context.spans.len());
        println!("Events recorded: {}", context.events.len());

        if !context.spans.is_empty() {
            println!("\nRecent spans:");
            for span in context.spans.iter().take(5) {
                println!("  - {} (trace: {:?})", span.data.name, span.data.trace_id);
            }
        }

        if !context.events.is_empty() {
            println!("\nRecent events:");
            for event in context.events.iter().take(5) {
                println!("  - {:?}: {}", event.data.level, event.data.message);
            }
        }
    }

    println!("\n========================================");
    println!("  Demo Complete!");
    println!("========================================\n");

    // Graceful shutdown
    handle.shutdown().await;

    Ok(())
}
