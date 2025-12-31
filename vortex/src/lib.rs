//! # Tardis Vortex
//!
//! The LLM inference engine for Tardis OS, built on Candle.
//!
//! Vortex provides:
//! - Model loading from SafeTensors/GGUF formats
//! - Multi-architecture support (Llama, Mistral, Phi, etc.)
//! - Quantization (Q4, Q8, FP16)
//! - KV-cache management
//! - Streaming token generation
//!
//! ## Example
//!
//! ```rust,no_run
//! use tardis_vortex::{Vortex, ModelLoadConfig, InferenceParams};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let vortex = Vortex::new()?;
//!
//!     let handle = vortex.load_model(
//!         "/models/phi-3-mini.safetensors",
//!         ModelLoadConfig::default(),
//!     ).await?;
//!
//!     let response = vortex.infer(
//!         handle,
//!         "Explain quantum computing",
//!         InferenceParams::default(),
//!     ).await?;
//!
//!     println!("{}", response);
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;
pub mod inference;
pub mod model;
pub mod tokenizer;

// Re-export main types
pub use config::{InferenceParams, ModelLoadConfig};
pub use error::{VortexError, VortexResult};
pub use inference::Vortex;
pub use model::{ModelHandle, ModelInfo, ModelRegistry};
pub use tokenizer::TokenizerService;
