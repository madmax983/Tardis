//! Telemetry subscriber configuration.
//!
//! Provides utilities for initializing the complete Tardis telemetry stack.

use crate::error::{TelemetryError, TelemetryResult};
use crate::types::Level;
use crate::userspace::layer::{SpanData, TardisLayer, TardisLayerConfig};
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "std")]
use crate::gallifrey::TelemetryStore;

/// Configuration for the Tardis telemetry system.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable Gallifrey storage for temporal queries.
    pub gallifrey_enabled: bool,

    /// Enable OpenTelemetry export.
    pub otlp_enabled: bool,

    /// OTLP endpoint URL.
    pub otlp_endpoint: Option<String>,

    /// OTLP batch size.
    pub otlp_batch_size: usize,

    /// Minimum level to record.
    pub min_level: Level,

    /// Enable console output.
    pub console_enabled: bool,

    /// Enable JSON formatting for console output.
    pub json_output: bool,

    /// Custom env filter directive.
    pub env_filter: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            gallifrey_enabled: true,
            otlp_enabled: false,
            otlp_endpoint: None,
            otlp_batch_size: 512,
            min_level: Level::Info,
            console_enabled: true,
            json_output: false,
            env_filter: None,
        }
    }
}

impl TelemetryConfig {
    /// Creates a new configuration with Gallifrey storage enabled.
    #[must_use]
    pub fn with_gallifrey() -> Self {
        Self {
            gallifrey_enabled: true,
            ..Default::default()
        }
    }

    /// Creates a new configuration with OTLP export enabled.
    #[must_use]
    pub fn with_otlp(endpoint: impl Into<String>) -> Self {
        Self {
            otlp_enabled: true,
            otlp_endpoint: Some(endpoint.into()),
            ..Default::default()
        }
    }

    /// Creates a minimal configuration for testing.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            gallifrey_enabled: false,
            otlp_enabled: false,
            console_enabled: true,
            ..Default::default()
        }
    }
}

/// Handle to the telemetry system.
///
/// This handle keeps the telemetry system alive and provides access
/// to the telemetry store for queries.
pub struct TelemetryHandle {
    /// Gallifrey telemetry store.
    #[cfg(feature = "std")]
    pub store: Option<Arc<TelemetryStore>>,

    /// OTLP exporter handle.
    otlp_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TelemetryHandle {
    /// Gets a reference to the telemetry store.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn store(&self) -> Option<&Arc<TelemetryStore>> {
        self.store.as_ref()
    }

    /// Shuts down the telemetry system gracefully.
    pub async fn shutdown(self) {
        if let Some(handle) = self.otlp_handle {
            handle.abort();
            let _ = handle.await;
        }
    }
}

impl std::fmt::Debug for TelemetryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryHandle")
            .field("has_store", &self.store.is_some())
            .field("has_otlp", &self.otlp_handle.is_some())
            .finish()
    }
}

/// Initializes the Tardis telemetry system.
///
/// This sets up the complete telemetry stack including:
/// - Tracing subscriber with the Tardis layer
/// - Gallifrey storage (if enabled)
/// - OTLP export (if enabled)
/// - Console output (if enabled)
///
/// # Errors
///
/// Returns an error if subscriber initialization fails.
///
/// # Example
///
/// ```rust,ignore
/// use tardis_telemetry::{init, TelemetryConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TelemetryConfig::default();
///     let handle = init(config)?;
///
///     tracing::info!("Telemetry initialized!");
///
///     // Use handle.store() for temporal queries
///
///     Ok(())
/// }
/// ```
pub fn init(config: TelemetryConfig) -> TelemetryResult<TelemetryHandle> {
    // Create Gallifrey store if enabled
    #[cfg(feature = "std")]
    let gallifrey_store = if config.gallifrey_enabled {
        Some(Arc::new(TelemetryStore::new()))
    } else {
        None
    };

    // Create OTLP sender if enabled
    let (otlp_sender, otlp_handle) = if config.otlp_enabled {
        let (tx, rx) = tokio::sync::mpsc::channel::<SpanData>(10_000);
        let endpoint = config.otlp_endpoint.clone();
        let batch_size = config.otlp_batch_size;

        let handle = tokio::spawn(async move {
            // OTLP exporter would run here
            // For now, just drain the channel
            let mut rx = rx;
            while let Some(_span) = rx.recv().await {
                // TODO: Implement actual OTLP export
            }
        });

        (Some(tx), Some(handle))
    } else {
        (None, None)
    };

    // Build the Tardis layer
    let tardis_layer_config = TardisLayerConfig {
        gallifrey_enabled: config.gallifrey_enabled,
        otlp_enabled: config.otlp_enabled,
        min_level: config.min_level,
    };

    let mut tardis_layer = TardisLayer::with_config(tardis_layer_config);

    #[cfg(feature = "std")]
    if let Some(store) = &gallifrey_store {
        tardis_layer = tardis_layer.with_store(Arc::clone(store));
    }

    if let Some(sender) = otlp_sender {
        tardis_layer = tardis_layer.with_otlp_sender(sender);
    }

    // Build the env filter
    let env_filter = if let Some(filter) = &config.env_filter {
        EnvFilter::try_new(filter).map_err(|e| TelemetryError::Config(e.to_string()))?
    } else {
        EnvFilter::from_default_env().add_directive("tardis=debug".parse().unwrap())
    };

    // Build the subscriber
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(tardis_layer);

    // Add console layer if enabled
    if config.console_enabled {
        if config.json_output {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            registry.with(fmt_layer).try_init().map_err(
                |e: tracing_subscriber::util::TryInitError| {
                    TelemetryError::SubscriberInit(e.to_string())
                },
            )?;
        } else {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            registry.with(fmt_layer).try_init().map_err(
                |e: tracing_subscriber::util::TryInitError| {
                    TelemetryError::SubscriberInit(e.to_string())
                },
            )?;
        }
    } else {
        registry
            .try_init()
            .map_err(|e: tracing_subscriber::util::TryInitError| {
                TelemetryError::SubscriberInit(e.to_string())
            })?;
    }

    Ok(TelemetryHandle {
        #[cfg(feature = "std")]
        store: gallifrey_store,
        otlp_handle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests need to be run in isolation because
    // tracing subscriber can only be initialized once per process.

    #[test]
    fn config_defaults() {
        let config = TelemetryConfig::default();
        assert!(config.gallifrey_enabled);
        assert!(!config.otlp_enabled);
        assert!(config.console_enabled);
    }

    #[test]
    fn config_with_otlp() {
        let config = TelemetryConfig::with_otlp("http://localhost:4317");
        assert!(config.otlp_enabled);
        assert_eq!(
            config.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
    }
}
