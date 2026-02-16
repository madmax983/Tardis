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
//!     // 1. Initialize the engine
//!     let vortex = Vortex::new()?;
//!
//!     // 2. Load a model (SafeTensors format)
//!     // Note: This requires a valid model file at the specified path.
//!     let handle = vortex.load_model(
//!         "/path/to/models/phi-3-mini.safetensors",
//!         ModelLoadConfig::default(),
//!     ).await?;
//!
//!     // 3. Run inference
//!     let response = vortex.infer(
//!         handle,
//!         "Explain quantum computing",
//!         InferenceParams::default(),
//!     ).await?;
//!
//!     println!("Response: {}", response);
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;
pub mod inference;
pub mod loader;
pub mod model;
pub mod tokenizer;

// Re-export main types
pub use config::{InferenceParams, ModelLoadConfig};
pub use error::{VortexError, VortexResult};
pub use inference::Vortex;
pub use loader::ModelPreset;
pub use model::{ModelHandle, ModelInfo, ModelRegistry};
pub use tokenizer::TokenizerService;

use async_trait::async_trait;
use std::fmt::Debug;

/// Core service trait for Vortex.
#[async_trait]
pub trait VortexService: Send + Sync + Debug {
    /// Load a model from disk.
    async fn load_model(
        &self,
        path: &str,
        config: ModelLoadConfig,
    ) -> VortexResult<ModelHandle>;

    /// Load a preset model.
    async fn load_preset(
        &self,
        preset: ModelPreset,
        device: Option<&str>,
    ) -> VortexResult<ModelHandle>;

    /// Load the default test model.
    async fn load_default_test_model(&self) -> VortexResult<ModelHandle>;

    /// Unload a model.
    async fn unload_model(&self, handle: ModelHandle) -> VortexResult<()>;

    /// Run inference on a model.
    async fn infer(
        &self,
        handle: ModelHandle,
        prompt: &str,
        params: InferenceParams,
    ) -> VortexResult<String>;

    /// Generate embeddings for text.
    async fn embed(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<f32>>;

    /// List available models.
    fn list_models(&self) -> Vec<ModelInfo>;

    /// List loaded models and their handles.
    fn list_loaded_models(&self) -> Vec<(ModelHandle, ModelInfo)>;

    /// Get information about a specific model.
    fn model_info(&self, handle: ModelHandle) -> Option<ModelInfo>;

    /// Check if a model is loaded.
    fn is_model_loaded(&self, handle: ModelHandle) -> bool;
}

#[async_trait]
impl VortexService for Vortex {
    async fn load_model(
        &self,
        path: &str,
        config: ModelLoadConfig,
    ) -> VortexResult<ModelHandle> {
        self.load_model(path, config).await
    }

    async fn load_preset(
        &self,
        preset: ModelPreset,
        device: Option<&str>,
    ) -> VortexResult<ModelHandle> {
        self.load_preset(preset, device).await
    }

    async fn load_default_test_model(&self) -> VortexResult<ModelHandle> {
        self.load_default_test_model().await
    }

    async fn unload_model(&self, handle: ModelHandle) -> VortexResult<()> {
        self.unload_model(handle).await
    }

    async fn infer(
        &self,
        handle: ModelHandle,
        prompt: &str,
        params: InferenceParams,
    ) -> VortexResult<String> {
        self.infer(handle, prompt, params).await
    }

    async fn embed(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<f32>> {
        self.embed(handle, text).await
    }

    fn list_models(&self) -> Vec<ModelInfo> {
        self.list_models()
    }

    fn list_loaded_models(&self) -> Vec<(ModelHandle, ModelInfo)> {
        self.list_loaded_models()
    }

    fn model_info(&self, handle: ModelHandle) -> Option<ModelInfo> {
        self.model_info(handle)
    }

    fn is_model_loaded(&self, handle: ModelHandle) -> bool {
        self.is_model_loaded(handle)
    }
}
