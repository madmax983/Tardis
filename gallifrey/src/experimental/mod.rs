//! Experimental features that are not yet stable.
//!
//! # Features
//! - `heatmap`: Temporal heatmap generation for visualizing entity history.
//! - `historian`: Analysis of temporal anomalies (Retcons and Prophecies).

/// Temporal heatmap visualization.
pub mod heatmap;

/// Temporal anomaly analysis.
#[cfg(feature = "nova")]
pub mod historian;
