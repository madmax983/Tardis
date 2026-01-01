//! Main Vortex inference runtime.

use crate::config::{InferenceParams, ModelLoadConfig};
use crate::error::{VortexError, VortexResult};
use crate::loader::{
    download_preset, find_model_file, load_model_weights, parse_model_config, DeviceSpec,
    LoadedModel, ModelConfig, ModelPreset,
};
use crate::model::{ModelHandle, ModelInfo, ModelRegistry};
use crate::tokenizer::TokenizerService;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tardis_common::traits::{InferenceParams as TraitParams, ModelInfo as TraitInfo, VortexService};
use tokio::task;
use tracing::{info, instrument};

/// The main Vortex inference engine.
pub struct Vortex {
    /// Model registry.
    registry: Arc<ModelRegistry>,
    /// Tokenizer service.
    tokenizers: Arc<TokenizerService>,
    /// Loaded models by handle.
    loaded_models: RwLock<HashMap<ModelHandle, LoadedModel>>,
    /// Model configurations by handle.
    model_configs: RwLock<HashMap<ModelHandle, ModelConfig>>,
}

impl std::fmt::Debug for Vortex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vortex")
            .field("registry", &self.registry)
            .field("tokenizers", &self.tokenizers)
            .field(
                "loaded_models_count",
                &self.loaded_models.read().map(|m| m.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl Vortex {
    /// Create a new Vortex instance.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn new() -> VortexResult<Self> {
        info!("Initializing Vortex inference engine");

        Ok(Self {
            registry: Arc::new(ModelRegistry::new()),
            tokenizers: Arc::new(TokenizerService::new()),
            loaded_models: RwLock::new(HashMap::new()),
            model_configs: RwLock::new(HashMap::new()),
        })
    }

    /// Load a model from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be loaded.
    #[instrument(skip(self, config))]
    pub async fn load_model(&self, path: &str, config: ModelLoadConfig) -> VortexResult<ModelHandle> {
        let path_buf = std::path::PathBuf::from(path);

        if !path_buf.exists() {
            return Err(VortexError::ModelNotFound {
                path: path.to_string(),
            });
        }

        info!("Loading model from {}", path);

        // Parse model configuration (async)
        let model_config = parse_model_config(&path_buf).await?;

        // Create device specification
        let device_spec = DeviceSpec::parse(&config.device);

        // Load model weights in blocking task to avoid stalling async runtime
        let load_path = path_buf.clone();
        let load_config = model_config.clone();
        let use_mmap = config.use_mmap;
        let loaded_model = task::spawn_blocking(move || {
            load_model_weights(&load_path, &load_config, &device_spec, use_mmap)
        })
        .await
        .map_err(|e| VortexError::LoadFailed(format!("Weight loading task failed: {e}")))??;

        // Get memory usage from loaded model
        let memory = loaded_model.memory_bytes();

        // Create model info for registry
        let info = self.create_model_info(&path_buf, &model_config);
        self.registry.register(path_buf.clone(), info)?;

        // Mark as loaded and get handle
        let handle = self.registry.mark_loaded(&path_buf, memory)?;

        // Store the loaded model
        {
            let mut models = self.loaded_models.write().map_err(|_| {
                VortexError::LockPoisoned { context: "storing loaded model" }
            })?;
            models.insert(handle, loaded_model);
        }

        // Store the config
        {
            let mut configs = self.model_configs.write().map_err(|_| {
                VortexError::LockPoisoned { context: "storing model config" }
            })?;
            configs.insert(handle, model_config);
        }

        // Load tokenizer using shared helper
        let tokenizer_path = find_model_file(&path_buf, "tokenizer.json");

        if tokenizer_path.exists() {
            self.tokenizers.load(handle, &tokenizer_path)?;
            info!("Tokenizer loaded from {}", tokenizer_path.display());
        } else {
            info!("No tokenizer.json found, tokenization will not be available");
        }

        info!("Model loaded successfully: {} ({} bytes)", handle, memory);

        Ok(handle)
    }

    /// Load a preset model, downloading it from `HuggingFace` Hub if necessary.
    ///
    /// This is a convenience method that handles downloading and loading
    /// commonly-used models in one step.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or load fails.
    #[instrument(skip(self))]
    pub async fn load_preset(
        &self,
        preset: ModelPreset,
        device: Option<&str>,
    ) -> VortexResult<ModelHandle> {
        info!(
            "Loading preset model: {} ({})",
            preset.display_name(),
            preset.repo_id()
        );

        // Download the model (or use cached version)
        let model_path = download_preset(preset)?;

        info!("Model downloaded to: {}", model_path.display());

        // Load the model
        let config = ModelLoadConfig {
            device: device.unwrap_or("cpu").to_string(),
            quantization: None,
            max_context_length: None,
            use_mmap: true,
            tensor_parallel: 1,
        };

        self.load_model(model_path.to_string_lossy().as_ref(), config)
            .await
    }

    /// Load the default test model (`TinyLlama`).
    ///
    /// This is a convenience method for testing and development.
    ///
    /// # Errors
    ///
    /// Returns an error if the download or load fails.
    pub async fn load_default_test_model(&self) -> VortexResult<ModelHandle> {
        self.load_preset(ModelPreset::default_test_model(), None)
            .await
    }

    /// Create model info from config.
    fn create_model_info(&self, path: &Path, config: &ModelConfig) -> ModelInfo {
        ModelInfo {
            name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: path.to_path_buf(),
            architecture: config.architecture,
            parameters: config.estimate_parameters(),
            context_length: config.max_seq_len,
            quantization: crate::model::Quantization::F16, // TODO: detect from weights
            loaded: false,
            memory_bytes: None,
            num_layers: config.num_layers,
            hidden_size: config.hidden_size,
            num_heads: config.num_heads,
            vocab_size: config.vocab_size,
        }
    }

    /// Unload a model.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be unloaded.
    pub async fn unload_model(&self, handle: ModelHandle) -> VortexResult<()> {
        if !self.registry.is_valid(handle) {
            return Err(VortexError::InvalidHandle(handle.raw()));
        }

        info!("Unloading model {}", handle);

        // Remove loaded model
        {
            let mut models = self.loaded_models.write().map_err(|_| {
                VortexError::LockPoisoned { context: "removing loaded model" }
            })?;
            models.remove(&handle);
        }

        // Remove config
        {
            let mut configs = self.model_configs.write().map_err(|_| {
                VortexError::LockPoisoned { context: "removing model config" }
            })?;
            configs.remove(&handle);
        }

        self.tokenizers.unload(handle)?;
        self.registry.mark_unloaded(handle)?;

        info!("Model {} unloaded successfully", handle);

        Ok(())
    }

    /// Run inference on a model.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails.
    #[instrument(skip(self, params))]
    pub async fn infer(
        &self,
        handle: ModelHandle,
        prompt: &str,
        params: InferenceParams,
    ) -> VortexResult<String> {
        if !self.registry.is_valid(handle) {
            return Err(VortexError::InvalidHandle(handle.raw()));
        }

        info!(
            "Running inference: {} tokens max, temp={}",
            params.max_tokens, params.temperature
        );

        // TODO: Implement actual inference with Candle
        // For now, return a placeholder

        Ok(format!(
            "[Vortex] Inference placeholder for prompt: {}...",
            &prompt[..prompt.len().min(50)]
        ))
    }

    /// Generate embeddings for text.
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed(&self, handle: ModelHandle, text: &str) -> VortexResult<Vec<f32>> {
        if !self.registry.is_valid(handle) {
            return Err(VortexError::InvalidHandle(handle.raw()));
        }

        // TODO: Implement actual embedding with Candle
        // For now, return a placeholder vector

        let _tokens = self.tokenizers.encode(handle, text)?;

        Ok(vec![0.0; 4096]) // Placeholder
    }

    /// List available models.
    pub fn list_models(&self) -> Vec<ModelInfo> {
        self.registry.list()
    }

    /// Get information about a specific model.
    pub fn model_info(&self, handle: ModelHandle) -> Option<ModelInfo> {
        let path = self.registry.get_path(handle)?;
        self.registry.get_info(&path)
    }

    /// Get a reference to a loaded model.
    ///
    /// This is used internally for inference operations.
    #[allow(dead_code)] // Will be used in inference implementation
    pub(crate) fn get_loaded_model(&self, _handle: ModelHandle) -> VortexResult<std::sync::RwLockReadGuard<'_, HashMap<ModelHandle, LoadedModel>>> {
        self.loaded_models.read().map_err(|_| {
            VortexError::LockPoisoned { context: "reading loaded models" }
        })
    }

    /// Check if a model is loaded.
    #[must_use]
    pub fn is_model_loaded(&self, handle: ModelHandle) -> bool {
        self.loaded_models
            .read()
            .map(|models| models.contains_key(&handle))
            .unwrap_or(false)
    }
}

/// Implement the `VortexService` trait for integration with other subsystems.
#[async_trait]
impl VortexService for Vortex {
    async fn load_model(
        &self,
        path: &str,
        config: tardis_common::traits::ModelLoadConfig,
    ) -> tardis_common::Result<tardis_common::ModelHandle> {
        let local_config = ModelLoadConfig {
            device: config.device.unwrap_or_else(|| "cpu".to_string()),
            quantization: config.quantization,
            max_context_length: config.max_context_length,
            use_mmap: config.use_mmap,
            tensor_parallel: 1,
        };

        let handle = self.load_model(path, local_config).await.map_err(|e| {
            tardis_common::Error::ModelLoadFailed {
                name: path.to_string(),
                reason: e.to_string(),
            }
        })?;

        Ok(tardis_common::ModelHandle::new(handle.raw()))
    }

    async fn unload_model(&self, handle: tardis_common::ModelHandle) -> tardis_common::Result<()> {
        let local_handle = ModelHandle::new(handle.raw());
        self.unload_model(local_handle)
            .await
            .map_err(|e| tardis_common::Error::Internal(e.to_string()))
    }

    async fn infer(
        &self,
        handle: tardis_common::ModelHandle,
        prompt: &str,
        params: TraitParams,
    ) -> tardis_common::Result<String> {
        let local_handle = ModelHandle::new(handle.raw());
        let local_params = InferenceParams {
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: params.top_k,
            max_tokens: params.max_tokens,
            stop_sequences: params.stop_sequences,
            repetition_penalty: 1.1,
            seed: None,
        };

        self.infer(local_handle, prompt, local_params)
            .await
            .map_err(|e| tardis_common::Error::InferenceFailed {
                reason: e.to_string(),
            })
    }

    async fn embed(
        &self,
        handle: tardis_common::ModelHandle,
        text: &str,
    ) -> tardis_common::Result<Vec<f32>> {
        let local_handle = ModelHandle::new(handle.raw());
        self.embed(local_handle, text)
            .await
            .map_err(|e| tardis_common::Error::InferenceFailed {
                reason: e.to_string(),
            })
    }

    async fn list_models(&self) -> tardis_common::Result<Vec<TraitInfo>> {
        Ok(self
            .list_models()
            .into_iter()
            .map(|m| TraitInfo {
                name: m.name,
                architecture: m.architecture.to_string(),
                parameters: m.parameters,
                context_length: m.context_length,
                loaded: m.loaded,
                memory_bytes: m.memory_bytes,
            })
            .collect())
    }

    async fn model_info(&self, handle: tardis_common::ModelHandle) -> tardis_common::Result<TraitInfo> {
        let local_handle = ModelHandle::new(handle.raw());
        self.model_info(local_handle)
            .map(|m| TraitInfo {
                name: m.name,
                architecture: m.architecture.to_string(),
                parameters: m.parameters,
                context_length: m.context_length,
                loaded: m.loaded,
                memory_bytes: m.memory_bytes,
            })
            .ok_or_else(|| tardis_common::Error::InvalidModelHandle(handle.raw()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vortex_creation() {
        let vortex = Vortex::new();
        assert!(vortex.is_ok());
    }

    #[tokio::test]
    async fn test_load_model_not_found() {
        let vortex = Vortex::new().unwrap();
        let config = ModelLoadConfig::default();

        let result = vortex.load_model("/nonexistent/model/path", config).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VortexError::ModelNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_unload_invalid_handle() {
        let vortex = Vortex::new().unwrap();
        let invalid_handle = ModelHandle::new(999);

        let result = vortex.unload_model(invalid_handle).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VortexError::InvalidHandle(_)
        ));
    }

    #[tokio::test]
    async fn test_infer_invalid_handle() {
        let vortex = Vortex::new().unwrap();
        let invalid_handle = ModelHandle::new(999);

        let result = vortex
            .infer(invalid_handle, "test", InferenceParams::default())
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VortexError::InvalidHandle(_)
        ));
    }

    #[tokio::test]
    async fn test_embed_invalid_handle() {
        let vortex = Vortex::new().unwrap();
        let invalid_handle = ModelHandle::new(999);

        let result = vortex.embed(invalid_handle, "test").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VortexError::InvalidHandle(_)
        ));
    }

    #[test]
    fn test_is_model_loaded_false_for_invalid() {
        let vortex = Vortex::new().unwrap();
        let invalid_handle = ModelHandle::new(999);

        assert!(!vortex.is_model_loaded(invalid_handle));
    }

    #[test]
    fn test_model_info_none_for_invalid() {
        let vortex = Vortex::new().unwrap();
        let invalid_handle = ModelHandle::new(999);

        assert!(vortex.model_info(invalid_handle).is_none());
    }
}
