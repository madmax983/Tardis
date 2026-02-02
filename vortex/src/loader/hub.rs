//! `HuggingFace` Hub integration for model downloads.
//!
//! This module handles downloading models from the `HuggingFace` Hub
//! with caching support.

use crate::error::{VortexError, VortexResult};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::path::PathBuf;
use tracing::{info, instrument};

/// Known model presets for easy loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPreset {
    /// `TinyLlama` 1.1B - Small, fast, good for testing.
    TinyLlama,
    /// `SmolLM` 135M - Very small, fastest, minimal quality.
    SmolLm135M,
    /// `SmolLM` 360M - Small, fast, decent quality.
    SmolLm360M,
    /// Phi-2 2.7B - Good quality, moderate size.
    Phi2,
    /// Llama 3.2 1B - Good quality, small size.
    Llama3_2_1B,
}

impl ModelPreset {
    /// Get the `HuggingFace` model ID for this preset.
    #[must_use]
    pub const fn repo_id(&self) -> &'static str {
        match self {
            Self::TinyLlama => "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
            Self::SmolLm135M => "HuggingFaceTB/SmolLM-135M",
            Self::SmolLm360M => "HuggingFaceTB/SmolLM-360M",
            Self::Phi2 => "microsoft/phi-2",
            Self::Llama3_2_1B => "meta-llama/Llama-3.2-1B",
        }
    }

    /// Get the default model preset for testing.
    #[must_use]
    pub const fn default_test_model() -> Self {
        Self::TinyLlama
    }

    /// Estimate model size in bytes.
    #[must_use]
    pub const fn estimated_size_bytes(&self) -> u64 {
        match self {
            Self::TinyLlama => 2_200_000_000,   // ~2.2 GB
            Self::SmolLm135M => 270_000_000,    // ~270 MB
            Self::SmolLm360M => 720_000_000,    // ~720 MB
            Self::Phi2 => 5_600_000_000,        // ~5.6 GB
            Self::Llama3_2_1B => 2_500_000_000, // ~2.5 GB
        }
    }

    /// Get human-readable name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::TinyLlama => "TinyLlama 1.1B Chat",
            Self::SmolLm135M => "SmolLM 135M",
            Self::SmolLm360M => "SmolLM 360M",
            Self::Phi2 => "Phi-2 2.7B",
            Self::Llama3_2_1B => "Llama 3.2 1B",
        }
    }
}

/// Files needed for a complete model.
const MODEL_FILES: &[&str] = &["config.json", "tokenizer.json", "tokenizer_config.json"];

/// Download a model from `HuggingFace` Hub.
///
/// Returns the path to the cached model directory.
///
/// # Arguments
///
/// * `repo_id` - `HuggingFace` repository ID (e.g., "TinyLlama/TinyLlama-1.1B-Chat-v1.0")
///
/// # Errors
///
/// Returns an error if the download fails.
#[instrument]
pub fn download_model(repo_id: &str) -> VortexResult<PathBuf> {
    info!("Downloading model from HuggingFace: {}", repo_id);

    let api = Api::new().map_err(|e| {
        VortexError::LoadFailed(format!("Failed to initialize HuggingFace API: {e}"))
    })?;

    let repo = api.repo(Repo::new(repo_id.to_string(), RepoType::Model));

    // Download essential files first
    for file in MODEL_FILES {
        // Use a variable to avoid drop order issues in Rust 2024
        let result = repo.get(file);
        match result {
            Ok(path) => {
                info!("Downloaded {}: {}", file, path.display());
            }
            Err(e) => {
                // Some files are optional
                if *file == "tokenizer_config.json" {
                    info!("Optional file {} not found, continuing", file);
                } else {
                    return Err(VortexError::LoadFailed(format!(
                        "Failed to download {file}: {e}"
                    )));
                }
            }
        }
    }

    // Download model weights - try different patterns
    let weight_file = download_weights(&repo)?;
    info!("Model weights downloaded: {}", weight_file.display());

    // Return the directory containing the model
    let model_dir = weight_file.parent().ok_or_else(|| {
        VortexError::LoadFailed("Could not determine model directory".to_string())
    })?;

    Ok(model_dir.to_path_buf())
}

/// Download model weights, trying different file patterns.
fn download_weights(repo: &hf_hub::api::sync::ApiRepo) -> VortexResult<PathBuf> {
    // Try single file first
    if let Ok(path) = repo.get("model.safetensors") {
        return Ok(path);
    }

    // Try sharded files
    // First, check for index file
    if let Ok(index_path) = repo.get("model.safetensors.index.json") {
        info!("Found sharded model, downloading all shards...");

        // Read index to find all shards
        let index_content = std::fs::read_to_string(&index_path)
            .map_err(|e| VortexError::LoadFailed(format!("Failed to read index file: {e}")))?;

        let index: serde_json::Value = serde_json::from_str(&index_content)
            .map_err(|e| VortexError::LoadFailed(format!("Failed to parse index file: {e}")))?;

        // Get unique shard files
        if let Some(weight_map) = index.get("weight_map").and_then(|v| v.as_object()) {
            let mut shard_files: Vec<&str> =
                weight_map.values().filter_map(|v| v.as_str()).collect();
            shard_files.sort_unstable();
            shard_files.dedup();

            for shard in &shard_files {
                repo.get(shard).map_err(|e| {
                    VortexError::LoadFailed(format!("Failed to download shard {shard}: {e}"))
                })?;
                info!("Downloaded shard: {}", shard);
            }
        }

        return Ok(index_path);
    }

    // Try PyTorch format as fallback
    if repo.get("pytorch_model.bin").is_ok() {
        return Err(VortexError::LoadFailed(
            "Model only has PyTorch format (pytorch_model.bin), SafeTensors required".to_string(),
        ));
    }

    Err(VortexError::LoadFailed(
        "No SafeTensors weights found in repository".to_string(),
    ))
}

/// Download a preset model.
///
/// # Errors
///
/// Returns an error if the download fails.
pub fn download_preset(preset: ModelPreset) -> VortexResult<PathBuf> {
    info!(
        "Downloading preset model: {} ({})",
        preset.display_name(),
        preset.repo_id()
    );
    download_model(preset.repo_id())
}

/// Get the cached path for a model if it exists.
///
/// Returns `None` if the model is not cached.
#[must_use]
pub fn get_cached_model(repo_id: &str) -> Option<PathBuf> {
    let api = Api::new().ok()?;
    let repo = api.repo(Repo::new(repo_id.to_string(), RepoType::Model));

    // Check if config.json is cached (indicates model is downloaded)
    let config_path = repo.get("config.json").ok()?;
    config_path.parent().map(PathBuf::from)
}

/// Check if a preset model is already cached.
#[must_use]
pub fn is_preset_cached(preset: ModelPreset) -> bool {
    get_cached_model(preset.repo_id()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_repo_ids() {
        assert_eq!(
            ModelPreset::TinyLlama.repo_id(),
            "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
        );
        assert_eq!(
            ModelPreset::SmolLm135M.repo_id(),
            "HuggingFaceTB/SmolLM-135M"
        );
    }

    #[test]
    fn test_default_test_model() {
        assert_eq!(ModelPreset::default_test_model(), ModelPreset::TinyLlama);
    }
}
