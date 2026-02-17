# Tardis Telemetry 📡

> "I can see everything. All that is, all that was, all that ever could be."

**Telemetry** provides full-stack observability for Tardis OS, from the kernel ring buffer up to user-space application metrics. It unifies logging, tracing, and metrics collection into a single, cohesive system.

## 🏗️ Architecture

The telemetry stack is designed to work in both `no_std` (Kernel) and `std` (Userspace) environments.

```text
┌─────────────────────────────────────────┐
│           External Tools                │
│    Jaeger / Grafana / Prometheus        │
└─────────────────▲───────────────────────┘
                  │ OTLP (gRPC)
┌─────────────────┴───────────────────────┐
│          tardis-telemetry               │
│  [Userspace]                            │
│  • Tracing Subscriber (Logs/Spans)      │
│  • Metrics Registry (Counters/Gauges)   │
│  • Gallifrey Store (Historical Logs)    │
└─────────────────▲───────────────────────┘
                  │ Ring Buffer
┌─────────────────┴───────────────────────┐
│              Kernel                     │
│  [Kernel Space]                         │
│  • KernelLogger (printk)                │
│  • Serial Port Output                   │
└─────────────────────────────────────────┘
```

## 🚀 Usage

### Initialization
```rust
use tardis_telemetry::{init, TelemetryConfig};

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber and metrics exporter
    let _handle = init(TelemetryConfig::default())?;

    tracing::info!("Telemetry initialized!");
    Ok(())
}
```

### Metrics
We provide convenient macros for recording metrics:

```rust
use tardis_telemetry::{counter, gauge, histogram};

fn process_request() {
    // Increment a counter
    counter!("requests_total", "service" => "api").inc();

    // Record latency
    let start = std::time::Instant::now();
    // ... work ...
    histogram!("request_duration_ms").record(start.elapsed().as_millis() as f64);
}
```

## 🚩 Feature Flags

-   **`std`** *(Default)*: Enables userspace features (tracing-subscriber, Tokio runtime).
-   **`kernel`**: Enables `no_std` support for the OS kernel.
-   **`otlp`**: Enables OpenTelemetry Protocol (OTLP) exporters for integration with external observability platforms.
