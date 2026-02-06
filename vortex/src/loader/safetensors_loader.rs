//! `SafeTensors` model loader.

use super::config::ModelConfig;
use super::types::LoadedModel;
use crate::error::{VortexError, VortexResult};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::models::llama::{Llama, LlamaConfig};
use std::path::Path;
use tracing::{info, instrument};

/// Load a Llama model from `SafeTensors` format.
#[instrument(skip(config))]
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn load_llama_safetensors(
    model_path: &Path,
    config: &ModelConfig,
    device: &Device,
    dtype: DType,
) -> VortexResult<LoadedModel> {
    // Build LlamaConfig that matches candle-transformers' expected format
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
        eos_token_id: config
            .eos_token_id
            .map(candle_transformers::models::llama::LlamaEosToks::Single),
        rope_scaling: None,
        max_position_embeddings: config.max_seq_len,
        tie_word_embeddings: None,
    };

    // Convert to runtime config
    let runtime_config = llama_config.into_config(false);

    info!(
        "Creating Llama model: {} layers, {} hidden, {} heads",
        runtime_config.num_hidden_layers,
        runtime_config.hidden_size,
        runtime_config.num_attention_heads
    );

    // Find weight files
    let weight_files = find_safetensors_files(model_path)?;
    info!("Found {} weight file(s)", weight_files.len());

    // Create VarBuilder from weight files using memory mapping for performance
    info!("Loading weights with memory mapping");

    // SAFETY: Memory-mapped file loading requires the following invariants:
    // 1. Files must not be modified by other processes while mapped - the caller
    //    is responsible for ensuring model files are stable during loading.
    // 2. Files must remain valid for the lifetime of the VarBuilder/model - the
    //    LoadedModel owns the mapping and keeps files valid.
    // 3. The safetensors format includes checksums that candle validates during
    //    loading, protecting against corruption.
    // 4. We only read from the mapped memory, never write.
    #[allow(unsafe_code)]
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&weight_files, dtype, device)? };

    // Build the model
    info!("Building Llama model...");
    let model = Llama::load(vb, &runtime_config)
        .map_err(|e| VortexError::LoadFailed(format!("Failed to build Llama model: {e}")))?;

    info!("Llama model loaded successfully");

    Ok(LoadedModel::Llama {
        model,
        config: runtime_config,
        device: device.clone(),
        dtype,
    })
}

/// Find `SafeTensors` weight files in a model directory.
///
/// This function uses blocking I/O. For async contexts, wrap the call
/// to `load_model_weights` in `spawn_blocking`.
fn find_safetensors_files(model_path: &Path) -> VortexResult<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    if model_path.is_file() {
        // Single file provided
        if model_path
            .extension()
            .is_some_and(|ext| ext == "safetensors")
        {
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
    fn test_find_safetensors_files_not_found() {
        let result = find_safetensors_files(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }
}
