//! OTLP (OpenTelemetry Protocol) exporter.

use crate::error::TelemetryResult;
use crate::userspace::layer::SpanData;
use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration for the OTLP exporter.
#[derive(Debug, Clone)]
pub struct OtlpConfig {
    /// OTLP endpoint URL.
    pub endpoint: String,

    /// Maximum batch size before flushing.
    pub batch_size: usize,

    /// Maximum time to wait before flushing.
    pub flush_interval: Duration,

    /// Request timeout.
    pub timeout: Duration,

    /// Optional headers to include in requests.
    pub headers: Vec<(String, String)>,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            batch_size: 512,
            flush_interval: Duration::from_secs(5),
            timeout: Duration::from_secs(10),
            headers: Vec::new(),
        }
    }
}

impl OtlpConfig {
    /// Creates a new configuration with the given endpoint.
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            ..Default::default()
        }
    }

    /// Sets the batch size.
    #[must_use]
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Sets the flush interval.
    #[must_use]
    pub fn with_flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// Adds a header.
    #[must_use]
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}

/// OTLP exporter for traces.
///
/// Batches spans and exports them via OTLP gRPC.
pub struct OtlpExporter {
    config: OtlpConfig,
    sender: mpsc::Sender<SpanData>,
    handle: tokio::task::JoinHandle<()>,
}

impl OtlpExporter {
    /// Creates and starts a new OTLP exporter.
    pub async fn new(config: OtlpConfig) -> TelemetryResult<Self> {
        let (sender, receiver) = mpsc::channel(10_000);

        let export_config = config.clone();
        let handle = tokio::spawn(async move {
            Self::run_export_loop(receiver, export_config).await;
        });

        Ok(Self {
            config,
            sender,
            handle,
        })
    }

    /// Returns a sender for submitting spans.
    #[must_use]
    pub fn sender(&self) -> mpsc::Sender<SpanData> {
        self.sender.clone()
    }

    /// Shuts down the exporter gracefully.
    pub async fn shutdown(self) {
        drop(self.sender);
        let _ = self.handle.await;
    }

    /// Runs the export loop.
    async fn run_export_loop(mut receiver: mpsc::Receiver<SpanData>, config: OtlpConfig) {
        let mut batch = Vec::with_capacity(config.batch_size);
        let mut interval = tokio::time::interval(config.flush_interval);

        loop {
            tokio::select! {
                Some(span) = receiver.recv() => {
                    batch.push(span);
                    if batch.len() >= config.batch_size {
                        Self::flush_batch(&config, &mut batch).await;
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        Self::flush_batch(&config, &mut batch).await;
                    }
                }
                else => break,
            }
        }

        // Final flush
        if !batch.is_empty() {
            Self::flush_batch(&config, &mut batch).await;
        }
    }

    /// Flushes a batch of spans to the OTLP endpoint.
    async fn flush_batch(config: &OtlpConfig, batch: &mut Vec<SpanData>) {
        // TODO: Implement actual OTLP gRPC export
        // For now, just log and clear the batch

        tracing::debug!(
            endpoint = %config.endpoint,
            span_count = batch.len(),
            "Flushing span batch (OTLP export not yet implemented)"
        );

        batch.clear();
    }
}

impl std::fmt::Debug for OtlpExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtlpExporter")
            .field("config", &self.config)
            .finish()
    }
}
