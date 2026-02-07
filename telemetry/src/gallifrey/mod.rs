//! Gallifrey integration for telemetry storage.
//!
//! This module provides temporal storage of telemetry data using
//! Gallifrey's bi-temporal model. Spans and events are stored as
//! entities with full time-travel capability.
//!
//! # Features
//!
//! - Store spans as entities with bi-temporal tracking
//! - Store events linked to their parent spans
//! - Query "what was happening at time X"
//! - Get context around specific timestamps
//!
//! # Example
//!
//! ```rust,ignore
//! use tardis_telemetry::gallifrey::TelemetryStore;
//! use chrono::Utc;
//!
//! let store = TelemetryStore::new();
//!
//! // Query spans active at a specific time
//! let spans = store.spans_at(some_timestamp, None).await?;
//!
//! // Get context around an error
//! let context = store.context_around(error_time, 5000).await?;
//! ```

mod store;

pub use store::{TelemetryContext, TelemetryStore};
