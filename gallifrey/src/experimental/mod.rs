//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `entropy`: System entropy and stability metrics.
//! - `timeline`: "What-If" simulation overlay.

/// Temporal heatmap visualization.
pub mod heatmap;

/// System entropy and stability metrics.
pub mod entropy;

/// Timeline simulation for "What-If" scenarios.
pub mod timeline;
