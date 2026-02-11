//! Experimental features for Gallifrey.
//!
//! These features are unstable and hidden behind the `nova` feature flag.
//!
//! # Features
//!
//! - `heatmap`: Temporal heatmap visualization for TUI.

#[cfg(feature = "nova")]
/// Temporal heatmap visualization.
pub mod heatmap;
