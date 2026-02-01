//! Model loading infrastructure for Vortex.
//!
//! This module handles:
//! - Parsing model configuration (config.json)
//! - Device selection (CPU/CUDA/Metal)
//! - Loading model weights from SafeTensors/GGUF formats
//! - Creating Candle model instances
//! - Downloading models from `HuggingFace` Hub

mod config;
mod device;
mod hub;
mod weights;

use std::path::{Path, PathBuf};

pub use config::{ModelConfig, parse_model_config};
pub use device::{DeviceSpec, create_device};
pub use hub::{ModelPreset, download_model, download_preset, get_cached_model, is_preset_cached};
pub use weights::{LoadedModel, ModelFormat, load_model_weights};

/// Resolve a sibling file path relative to a model path.
///
/// If `model_path` is a directory, returns `model_path/filename`.
/// If `model_path` is a file, returns its parent directory joined with `filename`.
#[must_use]
pub fn find_model_file(model_path: &Path, filename: &str) -> PathBuf {
    if model_path.is_dir() {
        model_path.join(filename)
    } else {
        model_path
            .parent()
            .map_or_else(|| model_path.with_file_name(filename), |p| p.join(filename))
    }
}
