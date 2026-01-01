# ADR-008: Observability Architecture

## Status

Accepted

## Date

2025-01-01

## Context

Tardis OS positions itself as an AI-native operating system where understanding system behavior is critical. We need comprehensive observability to:

1. **Debug AI inference issues** - Track model loading, token generation, latency
2. **Time-travel debugging** - Leverage Gallifrey's bi-temporal model to answer "what was happening at time X?"
3. **Performance optimization** - Measure syscall latency, memory usage, inference throughput
4. **Production monitoring** - Export to standard tools (Jaeger, Grafana, Prometheus)
5. **Kernel diagnostics** - Observe boot sequence, panics, interrupts in a no_std environment

The challenge is supporting both the no_std kernel environment and feature-rich userspace while maintaining sub-microsecond overhead for kernel operations.

## Decision

We will create a new `tardis-telemetry` crate with a layered architecture:

### 1. Core Types (no_std compatible)

Shared types that work in both kernel and userspace:
- `TraceId`, `SpanId` - Distributed tracing identifiers
- `Level` - Severity levels (Trace, Debug, Info, Warn, Error)
- `Subsystem` - Component identifiers (Kernel, Vortex, Gallifrey, etc.)
- `EventType` - Typed events per subsystem
- `TelemetryEntry` - Compact binary format for ring buffer

### 2. Kernel Layer (no_std)

- **Ring Buffer**: Lock-free SPSC buffer for minimal overhead, readable from userspace
- **KernelLogger**: Implements `log::Log` trait, writes to ring buffer
- **Serial Output**: COM1 for early boot and panic messages

### 3. Userspace Layer (std)

- **TardisLayer**: Custom `tracing::Layer` implementation
- **MetricsRegistry**: Thread-safe counters, gauges, histograms
- **Subscriber**: Configurable subscriber stack with filtering
- **Drainer**: Reads kernel ring buffer, converts to tracing spans

### 4. Gallifrey Integration

Store telemetry as entities in Gallifrey's knowledge graph:
- Spans become `Entity` with type "Span" and bi-temporal tracking
- Events become `Entity` with type "Event"
- Relationships: `CHILD_OF` (span hierarchy), `CONTAINS` (span → events)

This enables temporal queries like:
- "Show all spans active at 2:45 PM"
- "What was the system doing when this error occurred?"
- "Replay the last 5 minutes of activity"

### 5. OpenTelemetry Export

- OTLP gRPC exporter for traces
- Prometheus format for metrics
- Async batching for performance

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    EXTERNAL TOOLS                       │
│         Jaeger / Grafana / Prometheus                   │
└────────────────────────▲────────────────────────────────┘
                         │ OTLP/Prometheus
┌────────────────────────┴────────────────────────────────┐
│                  tardis-telemetry                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ TardisLayer  │  │   Metrics    │  │    OTLP      │  │
│  │ (tracing)    │  │   Registry   │  │   Exporter   │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘  │
│         └─────────────────┴──────────────┐             │
│                                          ▼             │
│  ┌─────────────────────────────────────────────────┐   │
│  │         Gallifrey TelemetryStore                │   │
│  │  Spans/Events as Entities with BiTemporal       │   │
│  └─────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────┘
           ▲ Ring Buffer (shared memory)
┌──────────┴─────────────────────────────────────────────┐
│                      KERNEL                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │ KernelLogger│  │ RingBuffer  │  │   Serial    │    │
│  │ (log::Log)  │  │ (lock-free) │  │   Output    │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
└────────────────────────────────────────────────────────┘
```

### Feature Flags

```toml
[features]
default = ["std"]
std = ["tracing", "tracing-subscriber", "tokio", "parking_lot"]
otlp = ["std", "opentelemetry", "opentelemetry-otlp"]
kernel = []  # no_std kernel support
```

## Consequences

### Positive

- **Unified observability** across kernel and userspace with consistent types
- **Time-travel debugging** via Gallifrey integration - unique capability
- **Standard tooling support** via OpenTelemetry export
- **Low overhead** in kernel via lock-free ring buffer
- **Feature-gated dependencies** keep kernel minimal
- **Structured telemetry** with typed subsystems and events

### Negative

- **Additional crate** increases build complexity
- **Ring buffer overhead** adds ~100ns per kernel log entry
- **Gallifrey storage growth** - need retention policies for telemetry entities
- **Learning curve** for custom `tracing::Layer` implementation

### Neutral

- Kernel continues using `log` crate (familiar API)
- Userspace continues using `tracing` crate (familiar API)
- May drive improvements to Gallifrey query performance

## Alternatives Considered

### Alternative 1: Use OpenTelemetry SDK directly

Use the official OpenTelemetry Rust SDK for all observability.

**Pros:**
- Industry standard, well-documented
- Built-in exporters for many backends
- Active community

**Cons:**
- Heavy dependencies, not no_std compatible
- No Gallifrey integration for time-travel queries
- Less control over performance characteristics

**Why not chosen:** OpenTelemetry SDK is too heavy for kernel use and doesn't integrate with Gallifrey's temporal model, which is a core differentiator.

### Alternative 2: Extend existing tracing usage

Just add more `#[instrument]` annotations and configure tracing-subscriber better.

**Pros:**
- Minimal new code
- Uses existing infrastructure
- Quick to implement

**Cons:**
- No kernel observability
- No Gallifrey integration
- No custom temporal queries
- No metrics support

**Why not chosen:** Doesn't address kernel observability or the time-travel debugging capability that makes Tardis unique.

### Alternative 3: Use eBPF for kernel observability

Implement kernel tracing via eBPF programs.

**Pros:**
- Very low overhead
- Can trace without code changes
- Industry-proven approach

**Cons:**
- Requires eBPF runtime in kernel (significant work)
- Complex toolchain
- Not available until kernel is more mature

**Why not chosen:** eBPF is a future optimization; we need a simpler solution now that works with our from-scratch kernel.

## References

- [Tracing crate documentation](https://docs.rs/tracing)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Lock-free ring buffer design](https://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue)
- [ADR-004: GallifreyDB Temporal Store](004-gallifreydb-temporal-store.md)
