//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `entropy`: Temporal stability analysis (Retcon/Prophecy detection).

/// Temporal stability analysis.
pub mod entropy;
/// Temporal heatmap visualization.
pub mod heatmap;
