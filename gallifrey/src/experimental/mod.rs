//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `entropy`: System entropy gauge for measuring physical and epistemic volatility.

/// Temporal heatmap visualization.
pub mod heatmap;

/// System entropy gauge.
#[cfg(feature = "nova")]
pub mod entropy;
