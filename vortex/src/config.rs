//! LLM Configuration types shared across Tardis OS.
//!
//! Provides configuration structs for model loading and inference.
//!
//! Note: These types have been moved to `tardis_common::llm` to reduce coupling.
//! This module re-exports them for backward compatibility.

pub use tardis_common::llm::{InferenceParams, ModelLoadConfig};
