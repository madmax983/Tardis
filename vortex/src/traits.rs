use crate::config::{InferenceParams, ModelLoadConfig};
use crate::error::VortexResult;
use crate::loader::ModelPreset;
use crate::model::{ModelHandle, ModelInfo};
use async_trait::async_trait;

/// The Vortex service trait.
#[async_trait]
pub trait VortexService: Send + Sync + std::fmt::Debug {
    /// Load a model from disk.
    async fn load_model(&self, path: &str, config: ModelLoadConfig) -> VortexResult<ModelHandle>;

    /// Load a preset model.
    async fn load_preset(
        &self,
        preset: ModelPreset,
        device: Option<&str>,
    ) -> VortexResult<ModelHandle>;

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

    /// Unload a model.
    async fn unload_model(&self, handle: ModelHandle) -> VortexResult<()>;

    /// Get information about a specific model.
    fn model_info(&self, handle: ModelHandle) -> Option<ModelInfo>;

    /// Check if a model is loaded.
    fn is_model_loaded(&self, handle: ModelHandle) -> bool;
}

#[async_trait]
impl VortexService for crate::inference::Vortex {
    async fn load_model(&self, path: &str, config: ModelLoadConfig) -> VortexResult<ModelHandle> {
        self.load_model(path, config).await
    }

    async fn load_preset(
        &self,
        preset: ModelPreset,
        device: Option<&str>,
    ) -> VortexResult<ModelHandle> {
        self.load_preset(preset, device).await
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

    async fn unload_model(&self, handle: ModelHandle) -> VortexResult<()> {
        self.unload_model(handle).await
    }

    fn model_info(&self, handle: ModelHandle) -> Option<ModelInfo> {
        self.model_info(handle)
    }

    fn is_model_loaded(&self, handle: ModelHandle) -> bool {
        self.is_model_loaded(handle)
    }
}
