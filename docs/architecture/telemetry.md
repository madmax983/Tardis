# Telemetry Architecture

> Full-stack observability for Tardis OS with temporal query support.

## Overview

The telemetry subsystem provides unified observability across the Tardis OS stack, from kernel boot to AI inference. It captures three types of data:

| Type | Description | Example |
|------|-------------|---------|
| **Traces** | Request flows with timing | `Vortex::infer` span with 150ms duration |
| **Metrics** | Numeric measurements | `inference_total: 1234`, `heap_used_bytes: 16MB` |
| **Events** | Structured log entries | `INFO vortex: Model loaded path="/models/llama.gguf"` |

What makes Tardis telemetry unique is **Gallifrey integration** - all telemetry is stored with bi-temporal tracking, enabling queries like "show me what was happening at 2:45 PM yesterday."

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL TOOLS                                     │
│                  Jaeger / Grafana / Prometheus / Custom                      │
└─────────────────────────────────▲───────────────────────────────────────────┘
                                  │ OTLP gRPC / Prometheus scrape
┌─────────────────────────────────┴───────────────────────────────────────────┐
│                           EXPORT LAYER                                       │
│  ┌───────────────────────┐  ┌───────────────────────┐                       │
│  │   OTLP Exporter       │  │  Prometheus Exporter  │                       │
│  │   - Span batching     │  │  - /metrics endpoint  │                       │
│  │   - Async gRPC        │  │  - Pull-based         │                       │
│  └───────────────────────┘  └───────────────────────┘                       │
└─────────────────────────────────▲───────────────────────────────────────────┘
                                  │
┌─────────────────────────────────┴───────────────────────────────────────────┐
│                         USERSPACE LAYER                                      │
│                                                                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐  │
│  │  TardisLayer    │  │ MetricsRegistry │  │   Ring Buffer Drainer      │  │
│  │                 │  │                 │  │                             │  │
│  │  tracing Layer  │  │  Counters       │  │  Reads kernel events       │  │
│  │  implementation │  │  Gauges         │  │  Converts to tracing spans │  │
│  │                 │  │  Histograms     │  │                             │  │
│  └────────┬────────┘  └────────┬────────┘  └──────────────▲──────────────┘  │
│           │                    │                          │                  │
│           └────────────────────┼──────────────────────────┘                  │
│                                │                                             │
│  ┌─────────────────────────────▼─────────────────────────────────────────┐  │
│  │                    Gallifrey TelemetryStore                            │  │
│  │                                                                        │  │
│  │   Spans stored as Entity { type: "Span", temporal: BiTemporalInterval }│  │
│  │   Events stored as Entity { type: "Event", ... }                       │  │
│  │   Relationships: CHILD_OF, CONTAINS                                    │  │
│  │                                                                        │  │
│  │   Temporal queries:                                                    │  │
│  │     - spans_at(valid_time, transaction_time)                          │  │
│  │     - context_around(timestamp, window_ms)                            │  │
│  │     - metric_history(name, from, to)                                  │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
                    ▲
                    │ Shared Memory Ring Buffer
                    │
┌───────────────────┴──────────────────────────────────────────────────────────┐
│                           KERNEL LAYER (no_std)                              │
│                                                                              │
│  ┌─────────────────┐  ┌─────────────────────────┐  ┌─────────────────────┐  │
│  │  KernelLogger   │  │     Ring Buffer         │  │   Serial Backend    │  │
│  │                 │  │                         │  │                     │  │
│  │  log::Log impl  │──│  Lock-free SPSC         │  │  COM1 @ 0x3F8       │  │
│  │  log::info!()   │  │  4096 entries           │  │  Early boot output  │  │
│  │  log::error!()  │  │  Cache-line aligned     │  │  Panic messages     │  │
│  └─────────────────┘  └─────────────────────────┘  └─────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Core Types

All types are defined in `telemetry/src/types.rs` and are `no_std` compatible.

### Identifiers

```rust
/// 128-bit trace identifier (W3C Trace Context compatible).
pub struct TraceId([u8; 16]);

/// 64-bit span identifier.
pub struct SpanId([u8; 8]);
```

### Severity Levels

```rust
#[repr(u8)]
pub enum Level {
    Trace = 0,  // Verbose debugging
    Debug = 1,  // Development info
    Info = 2,   // Normal operation
    Warn = 3,   // Potential issues
    Error = 4,  // Failures
}
```

### Subsystems

```rust
#[repr(u16)]
pub enum Subsystem {
    // Kernel (0-99)
    Kernel = 0,
    Memory = 1,
    Scheduler = 2,
    Interrupt = 3,
    Syscall = 4,

    // Userspace (100+)
    Vortex = 100,
    Gallifrey = 101,
    Chronos = 102,
    Shell = 103,
}
```

### Event Types

```rust
#[repr(u16)]
pub enum EventType {
    // Lifecycle
    SpanStart = 0,
    SpanEnd = 1,
    Log = 2,
    Metric = 3,

    // Kernel
    Boot = 100,
    Panic = 101,
    SyscallEntry = 102,
    SyscallExit = 103,
    PageFault = 104,

    // Vortex
    ModelLoad = 200,
    ModelUnload = 201,
    InferenceStart = 202,
    InferenceEnd = 203,
    TokenGenerated = 204,

    // Gallifrey
    QueryStart = 300,
    QueryEnd = 301,
    EntityInsert = 302,
    TimeTravel = 303,

    // Chronos
    RagQuery = 400,
    Retrieval = 401,
    Augmentation = 402,
}
```

### Telemetry Entry

Compact binary format for kernel ring buffer:

```rust
#[repr(C)]
pub struct TelemetryEntry {
    pub timestamp_ns: u64,      // Nanoseconds since boot
    pub level: Level,           // 1 byte
    pub subsystem: Subsystem,   // 2 bytes
    pub event_type: EventType,  // 2 bytes
    pub span_id: SpanId,        // 8 bytes
    pub trace_id: TraceId,      // 16 bytes
    pub payload_len: u16,       // 2 bytes
    // Total header: 39 bytes, padded to 40
    // Followed by payload bytes
}
```

## Kernel Telemetry

### Ring Buffer

The kernel uses a lock-free Single-Producer Single-Consumer (SPSC) ring buffer:

- **Size**: 4096 entries (power of 2 for efficient modulo)
- **Alignment**: Cache-line aligned slots (64 bytes) to prevent false sharing
- **Overflow**: Drops oldest entries when full (performance over completeness)
- **Read**: Userspace drainer reads via shared memory mapping

```rust
pub struct TelemetryRingBuffer {
    write_pos: AtomicUsize,  // Only kernel writes
    read_pos: AtomicUsize,   // Only userspace reads
    slots: [RingSlot; 4096],
}

impl TelemetryRingBuffer {
    /// Write entry (kernel side). Returns false if buffer full.
    pub fn try_write(&self, entry: &TelemetryEntry, payload: &[u8]) -> bool;

    /// Read entry (userspace side). Returns None if empty.
    pub fn try_read(&self) -> Option<(TelemetryEntry, Vec<u8>)>;
}
```

### Kernel Logger

Implements `log::Log` to capture all `log!` macro invocations:

```rust
impl log::Log for KernelLogger {
    fn log(&self, record: &log::Record) {
        let entry = TelemetryEntry::from_record(record);

        // Try ring buffer first
        if !self.ring_buffer.try_write(&entry, msg_bytes) {
            // Buffer full - write to serial as fallback
            self.serial.write(msg_bytes);
        }
    }
}
```

### Serial Output

Direct COM1 output for:
- Early boot messages (before ring buffer initialized)
- Panic messages (ring buffer may be corrupted)
- Debug output when configured

## Userspace Telemetry

### TardisLayer

Custom `tracing::Layer` that:
1. Captures span enter/exit with timing
2. Records events with full context
3. Stores to Gallifrey with bi-temporal tracking
4. Optionally forwards to OTLP exporter

```rust
impl<S> Layer<S> for TardisLayer {
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        // Create span entity with BiTemporalInterval
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        // Create event entity, link to current span
    }

    fn on_close(&self, id: Id, ctx: Context<S>) {
        // Complete span, record duration
    }
}
```

### Metrics Registry

Thread-safe metrics with atomic operations:

```rust
// Counters - monotonically increasing
counter!("syscall_total", Subsystem::Kernel).inc();
counter!("inference_total", Subsystem::Vortex).add(batch_size);

// Gauges - can go up or down
gauge!("heap_used_bytes", Subsystem::Memory).set(bytes);
gauge!("loaded_models", Subsystem::Vortex).inc();

// Histograms - latency distributions
histogram!("inference_latency_ms", Subsystem::Vortex).record(duration_ms);
```

## Gallifrey Integration

### Storage Model

Telemetry is stored as entities in Gallifrey's knowledge graph:

```rust
// Span entity
Entity {
    id: EntityId::new(),
    entity_type: "Span".to_string(),
    name: "Vortex::infer".to_string(),
    properties: json!({
        "trace_id": "abc123...",
        "span_id": "def456...",
        "duration_ns": 150_000_000,
        "attributes": { "model": "llama-7b", "tokens": 128 }
    }),
    temporal: BiTemporalInterval {
        valid_time: TimeRange::bounded(span_start, span_end),
        transaction_time: TimeRange::from_now(),
    },
    embedding: None,
    source: Some("telemetry".to_string()),
}

// Span hierarchy relationship
Relationship {
    relationship_type: "CHILD_OF",
    source: child_span_entity_id,
    target: parent_span_entity_id,
}

// Span contains event relationship
Relationship {
    relationship_type: "CONTAINS",
    source: span_entity_id,
    target: event_entity_id,
}
```

### Temporal Queries

```rust
// What spans were active at a specific time?
let spans = store.spans_at(
    valid_time: "2024-01-15T14:45:00Z".parse()?,
    transaction_time: None,  // Current knowledge
).await?;

// Get context around an error
let context = store.context_around(
    timestamp: error_time,
    window_ms: 5000,  // 5 seconds before/after
).await?;
// Returns: TelemetryContext { spans, events, metrics }

// Metric history over time
let history = store.metric_history(
    "inference_latency_ms",
    from: hour_ago,
    to: now,
).await?;
```

## OpenTelemetry Export

### OTLP Exporter

Exports spans via OTLP gRPC:

```rust
pub struct OtlpExporter {
    endpoint: String,
    batch_size: usize,      // Default: 512
    flush_interval: Duration, // Default: 5s
}
```

### Prometheus Metrics

Exposes `/metrics` endpoint:

```
# HELP tardis_inference_total Total inference requests
# TYPE tardis_inference_total counter
tardis_inference_total{subsystem="vortex"} 1234

# HELP tardis_inference_latency_ms Inference latency histogram
# TYPE tardis_inference_latency_ms histogram
tardis_inference_latency_ms_bucket{le="10"} 100
tardis_inference_latency_ms_bucket{le="50"} 450
tardis_inference_latency_ms_bucket{le="100"} 800
tardis_inference_latency_ms_bucket{le="+Inf"} 1234
```

## Standard Metrics

### Kernel

| Metric | Type | Description |
|--------|------|-------------|
| `syscall_total` | Counter | Total syscall invocations |
| `page_fault_total` | Counter | Page fault count |
| `context_switch_total` | Counter | Context switches |
| `heap_used_bytes` | Gauge | Kernel heap usage |
| `process_count` | Gauge | Active processes |
| `syscall_latency_ns` | Histogram | Syscall latency |

### Vortex

| Metric | Type | Description |
|--------|------|-------------|
| `inference_total` | Counter | Inference requests |
| `tokens_generated_total` | Counter | Tokens produced |
| `model_load_total` | Counter | Model loads |
| `loaded_models` | Gauge | Currently loaded models |
| `gpu_memory_bytes` | Gauge | GPU memory usage |
| `inference_latency_ms` | Histogram | End-to-end latency |
| `time_to_first_token_ms` | Histogram | TTFT |

### Gallifrey

| Metric | Type | Description |
|--------|------|-------------|
| `query_total` | Counter | Query count |
| `insert_total` | Counter | Entity insertions |
| `time_travel_total` | Counter | Temporal queries |
| `entity_count` | Gauge | Total entities |
| `query_latency_us` | Histogram | Query latency |

### Chronos

| Metric | Type | Description |
|--------|------|-------------|
| `rag_query_total` | Counter | RAG queries |
| `retrieval_total` | Counter | Retrieval operations |
| `retrieval_latency_ms` | Histogram | Retrieval time |

## Configuration

```rust
TelemetryConfig {
    // Kernel
    kernel: KernelConfig {
        ring_buffer_size: 4096,
        serial_enabled: true,
    },

    // Gallifrey storage
    gallifrey: GallifreyConfig {
        enabled: true,
        retention_days: 30,
    },

    // OpenTelemetry
    otlp: OtlpConfig {
        enabled: true,
        endpoint: "http://localhost:4317",
        batch_size: 512,
        flush_interval_secs: 5,
    },

    // Filtering
    filter: FilterConfig {
        min_level: Level::Info,
        subsystem_levels: hashmap!{
            Subsystem::Vortex => Level::Debug,
        },
    },
}
```

## Usage Examples

### Instrumenting Code

```rust
use tracing::{info, instrument, span, Level};

#[instrument(skip(self, params))]
pub async fn infer(&self, prompt: &str, params: InferenceParams) -> Result<String> {
    info!(prompt_len = prompt.len(), "Starting inference");

    let tokens = self.generate(prompt, params).await?;

    info!(token_count = tokens.len(), "Inference complete");
    Ok(tokens.join(""))
}
```

### Recording Metrics

```rust
use tardis_telemetry::metrics::{counter, gauge, histogram};

// In inference handler
counter!("inference_total").inc();
let start = Instant::now();

let result = do_inference().await;

histogram!("inference_latency_ms").record(start.elapsed().as_millis() as u64);
gauge!("loaded_models").set(registry.count() as i64);
```

### Temporal Queries

```rust
use tardis_telemetry::gallifrey::TelemetryStore;

// What was happening during the slow inference?
let context = store.context_around(slow_inference_time, 1000).await?;

for span in context.spans {
    println!("{}: {} ({}ms)", span.subsystem, span.name, span.duration_ms);
}

for event in context.events {
    println!("  {} {}: {}", event.level, event.timestamp, event.message);
}
```

## Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Kernel log write | < 100ns | Lock-free ring buffer |
| Span creation | < 1μs | Atomic operations only |
| Metric increment | < 50ns | Relaxed atomic ordering |
| Gallifrey store | < 100μs | Async, batched |
| OTLP export | Background | Non-blocking |

## See Also

- [ADR-008: Observability Architecture](../adr/008-observability-architecture.md)
- [ADR-004: GallifreyDB Temporal Store](../adr/004-gallifreydb-temporal-store.md)
- [Kernel Architecture](kernel.md)
