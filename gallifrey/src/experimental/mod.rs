//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `entropy`: Temporal entropy gauge for measuring system drift.

/// Temporal heatmap visualization.
pub mod heatmap;

/// Temporal entropy gauge (Retcon vs Prophecy).
pub mod entropy;
