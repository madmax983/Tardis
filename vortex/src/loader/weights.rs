//! Model weight loading.
//!
//! Handles loading weights from `SafeTensors` formats,
//! creating Candle model instances.

use super::config::ModelConfig;
use super::device::DeviceSpec;
use crate::error::{VortexError, VortexResult};
use crate::model::Architecture;
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Config as LlamaRuntimeConfig, Llama, LlamaConfig};
use std::path::Path;
use tracing::{info, instrument};

/// A loaded model, ready for inference.
///
/// This enum wraps different model architectures in a common interface.
pub enum LoadedModel {
    /// Llama family model (Llama, Llama2, Llama3).
    Llama {
        /// The Candle Llama model.
        model: Llama,
        /// Model configuration.
        config: LlamaRuntimeConfig,
        /// Device the model is loaded on.
        device: Device,
        /// Data type used.
        dtype: DType,
    },
    // Future: Add more architectures
    // Mistral { model: Mistral, config: MistralConfig, device: Device, dtype: DType },
    // Phi { model: Phi, config: PhiConfig, device: Device, dtype: DType },
}

// Manual Debug implementation since Llama doesn't implement Debug
impl std::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llama { config, device, dtype, .. } => {
                f.debug_struct("LoadedModel::Llama")
                    .field("config", config)
                    .field("device", device)
                    .field("dtype", dtype)
                    .finish()
            }
        }
    }
}

impl LoadedModel {
    /// Get the device this model is loaded on.
    #[must_use]
    pub const fn device(&self) -> &Device {
        match self {
            Self::Llama { device, .. } => device,
        }
    }

    /// Get the data type used by this model.
    #[must_use]
    pub const fn dtype(&self) -> DType {
        match self {
            Self::Llama { dtype, .. } => *dtype,
        }
    }

    /// Estimate memory usage in bytes.
    #[must_use]
    pub fn memory_bytes(&self) -> u64 {
        match self {
            Self::Llama { config, dtype, .. } => {
                let params = estimate_params_from_runtime_config(config);
                let bytes_per_param = match dtype {
                    DType::F32 => 4,
                    // All other types (F16, BF16, quantized) use 2 bytes
                    _ => 2,
                };
                params * bytes_per_param
            }
        }
    }
}

/// Estimate parameter count from a Llama runtime config.
///
/// This uses the same formula as `ModelConfig::estimate_parameters` but works
/// with the Candle runtime config type.
fn estimate_params_from_runtime_config(config: &LlamaRuntimeConfig) -> u64 {
    let hidden = config.hidden_size as u64;
    let layers = config.num_hidden_layers as u64;
    let vocab = config.vocab_size as u64;
    let intermediate = config.intermediate_size as u64;

    // Embedding: vocab_size * hidden_size
    let embedding = vocab * hidden;

    // Per layer:
    // - Attention: 4 * hidden_size^2 (Q, K, V, O projections)
    // - FFN: 3 * hidden_size * intermediate (gate, up, down)
    // - Norms: 2 * hidden_size
    let per_layer = 4 * hidden * hidden + 3 * hidden * intermediate + 2 * hidden;

    // Output: hidden_size * vocab_size (often tied with embedding)
    let output = hidden * vocab;

    embedding + layers * per_layer + output
}

/// Load model weights from disk.
///
/// # Arguments
///
/// * `model_path` - Path to model directory or weight file
/// * `config` - Parsed model configuration
/// * `device_spec` - Device to load on
/// * `_use_mmap` - Ignored; memory mapping is always used for performance
///
/// # Errors
///
/// Returns an error if weights cannot be loaded.
#[instrument(skip(config))]
pub fn load_model_weights(
    model_path: &Path,
    config: &ModelConfig,
    device_spec: &DeviceSpec,
    _use_mmap: bool,
) -> VortexResult<LoadedModel> {
    // Create the device
    let device = super::device::create_device(device_spec)?;

    // Determine dtype based on device
    let dtype = match &device {
        Device::Cpu => DType::F32, // CPU works best with F32
        _ => DType::F16,           // GPU can use F16
    };

    info!(
        "Loading {} model on {:?} with dtype {:?}",
        config.architecture, device_spec, dtype
    );

    match config.architecture {
        Architecture::Llama | Architecture::Mistral => {
            // Mistral uses same architecture as Llama in candle
            load_llama(model_path, config, &device, dtype)
        }
        arch => Err(VortexError::UnsupportedArchitecture(arch.to_string())),
    }
}

/// Load a Llama model.
fn load_llama(
    model_path: &Path,
    config: &ModelConfig,
    device: &Device,
    dtype: DType,
) -> VortexResult<LoadedModel> {
    // Build LlamaConfig that matches candle-transformers' expected format
    #[allow(clippy::cast_possible_truncation)]
    let llama_config = LlamaConfig {
        hidden_size: config.hidden_size,
        intermediate_size: config.intermediate_size.unwrap_or(config.hidden_size * 4),
        vocab_size: config.vocab_size,
        num_hidden_layers: config.num_layers,
        num_attention_heads: config.num_heads,
        num_key_value_heads: config.num_kv_heads,
        rms_norm_eps: config.rms_norm_eps,
        rope_theta: config.rope_theta as f32,
        bos_token_id: config.bos_token_id,
        eos_token_id: config.eos_token_id.map(candle_transformers::models::llama::LlamaEosToks::Single),
        rope_scaling: None,
        max_position_embeddings: config.max_seq_len,
        tie_word_embeddings: None,
    };

    // Convert to runtime config
    let runtime_config = llama_config.into_config(false);

    info!(
        "Creating Llama model: {} layers, {} hidden, {} heads",
        runtime_config.num_hidden_layers, runtime_config.hidden_size, runtime_config.num_attention_heads
    );

    // Find weight files
    let weight_files = find_weight_files(model_path)?;
    info!("Found {} weight file(s)", weight_files.len());

    // Create VarBuilder from weight files using memory mapping for performance
    info!("Loading weights with memory mapping");
    // Safety: The files are read-only and we don't modify them
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_files, dtype, device)? };

    // Build the model
    info!("Building Llama model...");
    let model = Llama::load(vb, &runtime_config).map_err(|e| {
        VortexError::LoadFailed(format!("Failed to build Llama model: {e}"))
    })?;

    info!("Llama model loaded successfully");

    Ok(LoadedModel::Llama {
        model,
        config: runtime_config,
        device: device.clone(),
        dtype,
    })
}

/// Find `SafeTensors` weight files in a model directory.
fn find_weight_files(model_path: &Path) -> VortexResult<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if model_path.is_file() {
        // Single file provided
        if model_path.extension().is_some_and(|ext| ext == "safetensors") {
            files.push(model_path.to_path_buf());
        } else {
            return Err(VortexError::LoadFailed(format!(
                "Unsupported file format: {}",
                model_path.display()
            )));
        }
    } else if model_path.is_dir() {
        // Search directory for weight files
        // First, try model.safetensors (single file models)
        let single_file = model_path.join("model.safetensors");
        if single_file.exists() {
            files.push(single_file);
        } else {
            // Look for sharded files: model-00001-of-00002.safetensors
            for entry in std::fs::read_dir(model_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "safetensors") {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Include sharded files or any .safetensors files
                    if name.starts_with("model") || name.contains("safetensors") {
                        files.push(path);
                    }
                }
            }
        }
    } else {
        return Err(VortexError::ModelNotFound {
            path: model_path.display().to_string(),
        });
    }

    if files.is_empty() {
        return Err(VortexError::LoadFailed(format!(
            "No SafeTensors files found in {}",
            model_path.display()
        )));
    }

    // Sort files to ensure consistent ordering for sharded models
    files.sort();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_params_from_runtime_config() {
        let config = LlamaRuntimeConfig {
            hidden_size: 4096,
            intermediate_size: 11008,
            vocab_size: 32000,
            num_hidden_layers: 32,
            num_attention_heads: 32,
            num_key_value_heads: 32,
            rms_norm_eps: 1e-5,
            rope_theta: 10000.0,
            use_flash_attn: false,
            bos_token_id: Some(1),
            eos_token_id: None,
            rope_scaling: None,
            max_position_embeddings: 4096,
            tie_word_embeddings: false,
        };

        let params = estimate_params_from_runtime_config(&config);
        // Should be around 7B
        assert!(params > 6_000_000_000);
        assert!(params < 8_000_000_000);
    }
}
