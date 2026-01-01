//! Batch processor for telemetry export.

use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration for the batch processor.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum batch size.
    pub max_size: usize,

    /// Maximum time to wait before flushing.
    pub max_delay: Duration,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_size: 512,
            max_delay: Duration::from_secs(5),
        }
    }
}

/// Generic batch processor.
///
/// Collects items and flushes them in batches based on size or time.
pub struct BatchProcessor<T> {
    config: BatchConfig,
    sender: mpsc::Sender<T>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl<T: Send + 'static> BatchProcessor<T> {
    /// Creates a new batch processor with a flush callback.
    pub fn new<F>(config: BatchConfig, on_flush: F) -> Self
    where
        F: Fn(Vec<T>) + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(config.max_size * 2);

        let batch_config = config.clone();
        let handle = tokio::spawn(async move {
            Self::run_loop(receiver, batch_config, on_flush).await;
        });

        Self {
            config,
            sender,
            handle: Some(handle),
        }
    }

    /// Submits an item for batching.
    ///
    /// Returns an error if the channel is full.
    pub fn submit(&self, item: T) -> Result<(), mpsc::error::TrySendError<T>> {
        self.sender.try_send(item)
    }

    /// Submits an item for batching, waiting if necessary.
    pub async fn submit_async(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.sender.send(item).await
    }

    /// Shuts down the batch processor.
    pub async fn shutdown(mut self) {
        drop(self.sender);
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
    }

    /// Runs the batching loop.
    async fn run_loop<F>(mut receiver: mpsc::Receiver<T>, config: BatchConfig, on_flush: F)
    where
        F: Fn(Vec<T>) + Send + 'static,
    {
        let mut batch = Vec::with_capacity(config.max_size);
        let mut interval = tokio::time::interval(config.max_delay);

        loop {
            tokio::select! {
                Some(item) = receiver.recv() => {
                    batch.push(item);
                    if batch.len() >= config.max_size {
                        on_flush(std::mem::take(&mut batch));
                        batch = Vec::with_capacity(config.max_size);
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        on_flush(std::mem::take(&mut batch));
                        batch = Vec::with_capacity(config.max_size);
                    }
                }
                else => break,
            }
        }

        // Final flush
        if !batch.is_empty() {
            on_flush(batch);
        }
    }
}

impl<T> std::fmt::Debug for BatchProcessor<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchProcessor")
            .field("config", &self.config)
            .finish()
    }
}
