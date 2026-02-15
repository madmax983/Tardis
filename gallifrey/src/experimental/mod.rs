//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `entropy`: System entropy and stability metrics.
//! - `time_capsule`: Export/Import bi-temporal knowledge subgraphs.

/// Temporal heatmap visualization.
pub mod heatmap;

/// System entropy and stability metrics.
pub mod entropy;

/// Time Capsule for knowledge graph export/import.
pub mod time_capsule;
